<?php

namespace PackForge\Laravel\Tests;

use Illuminate\Support\Facades\Http;
use Orchestra\Testbench\Attributes\DefineEnvironment;

class ExportRouteTest extends TestCase
{
    private function project(): array
    {
        return ['component_id' => 'fefco-0201', 'params' => ['L' => 300], 'board' => 'C-flute'];
    }

    public function test_route_proxies_exports_with_the_server_key(): void
    {
        Http::fake(['*/export' => Http::response('<svg/>', 200, [
            'Content-Type' => 'image/svg+xml',
            'Content-Disposition' => 'attachment; filename="fefco-0201.svg"',
        ])]);

        $this->withoutMiddleware(\Illuminate\Foundation\Http\Middleware\VerifyCsrfToken::class)
            ->postJson('/packforge/export', ['project' => $this->project(), 'format' => 'svg'])
            ->assertOk()
            ->assertHeader('Content-Type', 'image/svg+xml')
            ->assertHeader('Content-Disposition', 'attachment; filename="fefco-0201.svg"');

        Http::assertSent(fn ($r) => $r->hasHeader('Authorization', 'Bearer pk_test_123'));
    }

    public function test_route_forwards_options_but_never_a_browser_watermark(): void
    {
        config(['packforge.export_route.watermark' => 'Proof']);
        Http::fake(['*/export' => Http::response('%PDF-1.4', 200, ['Content-Type' => 'application/pdf'])]);

        $this->withoutMiddleware(\Illuminate\Foundation\Http\Middleware\VerifyCsrfToken::class)
            ->postJson('/packforge/export', [
                'project' => $this->project(),
                'format' => 'pdf',
                'options' => ['page' => 'a4', 'rotate' => 90, 'watermark' => 'hacked'],
            ])
            ->assertOk();

        Http::assertSent(fn ($r) => $r['options']['page'] === 'a4'
            && $r['options']['rotate'] === 90
            && $r['options']['watermark'] === 'Proof');
    }

    public function test_route_rejects_bad_options(): void
    {
        Http::fake();

        $this->withoutMiddleware(\Illuminate\Foundation\Http\Middleware\VerifyCsrfToken::class)
            ->postJson('/packforge/export', ['project' => $this->project(), 'format' => 'dxf', 'options' => ['rotate' => 45]])
            ->assertStatus(422)
            ->assertJsonValidationErrors(['options.rotate']);

        Http::assertNothingSent();
    }

    public function test_route_validates_input(): void
    {
        Http::fake();

        $this->withoutMiddleware(\Illuminate\Foundation\Http\Middleware\VerifyCsrfToken::class)
            ->postJson('/packforge/export', ['project' => $this->project(), 'format' => 'exe'])
            ->assertStatus(422)
            ->assertJsonValidationErrors(['format']);

        Http::assertNothingSent();
    }

    public function test_api_validation_errors_become_422(): void
    {
        Http::fake(['*' => Http::response([
            'code' => 'invalid_params', 'detail' => 'invalid parameters',
            'errors' => [['param' => 'L', 'message' => 'must be at least 40']],
        ], 400)]);

        $this->withoutMiddleware(\Illuminate\Foundation\Http\Middleware\VerifyCsrfToken::class)
            ->postJson('/packforge/export', ['project' => $this->project(), 'format' => 'dxf'])
            ->assertStatus(422)
            ->assertJsonPath('errors.L', 'must be at least 40');
    }

    public function test_route_is_in_the_web_group_so_csrf_and_sessions_apply(): void
    {
        // Laravel skips CSRF verification while running tests, so check the route instead.
        $route = app('router')->getRoutes()->getByName('packforge.export');

        $this->assertNotNull($route);
        $this->assertSame(['POST'], array_values(array_diff($route->methods(), ['HEAD'])));
        $this->assertContains('web', $route->gatherMiddleware());
        $this->assertContains('throttle:60,1', $route->gatherMiddleware());
    }

    #[DefineEnvironment('disableExportRoute')]
    public function test_route_can_be_disabled(): void
    {
        $this->assertNull(app('router')->getRoutes()->getByName('packforge.export'));
    }

    protected function disableExportRoute($app): void
    {
        $app['config']->set('packforge.export_route.enabled', false);
    }
}
