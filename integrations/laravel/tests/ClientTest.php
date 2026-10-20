<?php

namespace PackForge\Laravel\Tests;

use Illuminate\Http\Client\Request;
use Illuminate\Support\Facades\Http;
use Illuminate\Support\Facades\Storage;
use PackForge\Laravel\Exceptions\AuthenticationException;
use PackForge\Laravel\Exceptions\ConnectionException;
use PackForge\Laravel\Exceptions\NotFoundException;
use PackForge\Laravel\Exceptions\PackForgeException;
use PackForge\Laravel\Exceptions\RateLimitedException;
use PackForge\Laravel\Exceptions\ValidationException;
use PackForge\Laravel\Facades\PackForge;

class ClientTest extends TestCase
{
    public function test_render_sends_the_project_and_parses_stats(): void
    {
        Http::fake(['api.packforge.test/v1/render' => Http::response(self::renderResponse())]);

        $result = PackForge::component('fefco-0201')
            ->params(['L' => 300, 'W' => 200])
            ->param('H', 150)
            ->board('C-flute')
            ->render();

        $this->assertSame(1051.0, $result->stats->blankWidth);
        $this->assertSame(['Width is larger than length.'], $result->warnings());
        Http::assertSentCount(1);
        [$request] = Http::recorded()[0];
        $this->assertSame('https://api.packforge.test/v1/render', $request->url());
        $this->assertSame('POST', $request->method());
        $this->assertTrue($request->hasHeader('Authorization', 'Bearer pk_test_123'));
        $this->assertSame([
            'component_id' => 'fefco-0201',
            'params' => ['L' => 300, 'W' => 200, 'H' => 150],
            'board' => 'C-flute',
            'units' => 'mm',
        ], json_decode($request->body(), true)['project']);
    }

    public function test_builder_is_immutable_and_supports_custom_boards(): void
    {
        $base = PackForge::component('fefco-0201')->params(['L' => 300]);
        $custom = $base->caliper(3.2)->units('in');

        $this->assertSame('C-flute', $base->toArray()['board']);
        $this->assertSame(['id' => 'custom', 'name' => 'Custom', 'caliper_mm' => 3.2], $custom->toArray()['board']);
        $this->assertSame('in', $custom->toArray()['units']);
    }

    public function test_export_returns_a_file_with_filename_and_etag(): void
    {
        Storage::fake('local');
        Http::fake(['api.packforge.test/v1/export' => Http::response("  0\nSECTION\n  0\nEOF\n", 200, [
            'Content-Type' => 'application/dxf',
            'Content-Disposition' => 'attachment; filename="fefco-0201.dxf"',
            'ETag' => '"abc"',
        ])]);

        $file = PackForge::component('fefco-0201')->params(['L' => 300])->dxf();

        $this->assertSame('fefco-0201.dxf', $file->filename);
        $this->assertSame('application/dxf', $file->mimeType);
        $this->assertSame('"abc"', $file->etag);
        $this->assertStringEndsWith("EOF\n", $file->contents());

        $file->store('orders/1/box.dxf', 'local');
        Storage::disk('local')->assertExists('orders/1/box.dxf');

        $response = $file->toResponse(request());
        $this->assertSame('attachment; filename="fefco-0201.dxf"', $response->headers->get('Content-Disposition'));
        Http::assertSent(fn (Request $r) => $r['format'] === 'dxf' && $r->hasHeader('Accept', '*/*'));
    }

    public function test_invalid_params_raise_a_validation_exception(): void
    {
        Http::fake(['*' => Http::response([
            'type' => 'https://packforge.dev/problems/invalid-params', 'title' => 'Invalid parameters',
            'status' => 400, 'detail' => 'invalid parameters: L: must be at least 40', 'code' => 'invalid_params',
            'errors' => [['param' => 'L', 'message' => 'must be at least 40']],
        ], 400, ['Content-Type' => 'application/problem+json'])]);

        try {
            PackForge::component('fefco-0201')->params(['L' => 5])->render();
            $this->fail('expected ValidationException');
        } catch (ValidationException $e) {
            $this->assertSame(['L' => 'must be at least 40'], $e->messages());
            $this->assertSame(400, $e->status);
        }
    }

    public function test_status_codes_map_to_specific_exceptions(): void
    {
        Http::fake([
            '*/components' => Http::response(['code' => 'unauthorized', 'detail' => 'API key not recognised'], 401),
            '*/components/nope' => Http::response(['code' => 'unknown_component', 'detail' => 'unknown component'], 404),
            '*/boards' => Http::response(['code' => 'rate_limited', 'detail' => 'slow down'], 429, ['Retry-After' => '7']),
        ]);

        $this->assertThrowsType(AuthenticationException::class, fn () => PackForge::components());
        $this->assertThrowsType(NotFoundException::class, fn () => PackForge::describe('nope'));

        try {
            PackForge::boards();
            $this->fail('expected RateLimitedException');
        } catch (RateLimitedException $e) {
            $this->assertSame(7, $e->retryAfter);
        }
        // 429 is retried (1 + 2 retries) before giving up.
        Http::assertSentCount(1 + 1 + 3);
    }

    public function test_server_errors_are_retried(): void
    {
        Http::fakeSequence('*')
            ->push(['code' => 'oops'], 503)
            ->push(self::renderResponse(), 200);

        $result = PackForge::render(['component_id' => 'fefco-0201', 'params' => [], 'board' => 'C-flute']);

        $this->assertSame(362.0, $result->stats->blankHeight);
        Http::assertSentCount(2);
    }

    public function test_connection_failures_are_wrapped(): void
    {
        Http::fake(fn () => throw new \Illuminate\Http\Client\ConnectionException('Connection refused'));

        $this->assertThrowsType(ConnectionException::class, fn () => PackForge::boards());
    }

    public function test_missing_key_is_reported_clearly(): void
    {
        config(['packforge.key' => null]);
        $this->app->forgetInstance(\PackForge\Laravel\Client::class);
        PackForge::clearResolvedInstances();
        Http::fake();

        try {
            PackForge::boards();
            $this->fail('expected PackForgeException');
        } catch (PackForgeException $e) {
            $this->assertSame('missing_key', $e->problemCode);
        }
        Http::assertNothingSent();
    }

    public function test_health_does_not_send_the_key(): void
    {
        Http::fake(['*/health' => Http::response(['status' => 'ok', 'version' => '0.1.0'])]);

        $this->assertSame('ok', PackForge::health()['status']);
        Http::assertSent(fn (Request $r) => ! $r->hasHeader('Authorization'));
    }

    /** @param  class-string<\Throwable>  $class */
    private function assertThrowsType(string $class, callable $fn): void
    {
        try {
            $fn();
        } catch (\Throwable $e) {
            $this->assertInstanceOf($class, $e);

            return;
        }
        $this->fail("expected {$class}");
    }
}
