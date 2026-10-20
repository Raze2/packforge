<?php

namespace PackForge\Laravel;

use Illuminate\Http\Client\ConnectionException as HttpConnectionException;
use Illuminate\Http\Client\Factory as HttpFactory;
use Illuminate\Http\Client\PendingRequest;
use Illuminate\Http\Client\RequestException;
use Illuminate\Http\Client\Response;
use PackForge\Laravel\Exceptions\ConnectionException;
use PackForge\Laravel\Exceptions\PackForgeException;
use Throwable;

/**
 * HTTP client for the PackForge API (pf-api). Usually used through the PackForge facade:
 *
 *     PackForge::component('fefco-0201')->params(['L' => 300, 'W' => 200, 'H' => 150])->export('dxf');
 */
class Client
{
    /**
     * @param  array{url?: string, key?: ?string, timeout?: float, retries?: int, retry_delay_ms?: int}  $config
     */
    public function __construct(
        protected HttpFactory $http,
        protected array $config,
    ) {}

    /** Starts a design for a component. */
    public function component(string $componentId): Design
    {
        return new Design($this, $componentId);
    }

    /**
     * Rebuilds a design from a saved project array (e.g. from the designer's pf:save event).
     *
     * @param  array<string, mixed>  $project
     */
    public function design(array $project): Design
    {
        return Design::fromArray($this, $project);
    }

    /** @return array{status: string, version: string} */
    public function health(): array
    {
        return $this->send(fn () => $this->request(auth: false)->get('health'))->json();
    }

    /** @return array<int, array<string, mixed>> */
    public function components(): array
    {
        return $this->send(fn () => $this->request()->get('components'))->json();
    }

    /** @return array<string, mixed> parameter schema */
    public function describe(string $componentId): array
    {
        return $this->send(fn () => $this->request()->get('components/'.rawurlencode($componentId)))->json();
    }

    /** @return array<int, array<string, mixed>> */
    public function boards(): array
    {
        return $this->send(fn () => $this->request()->get('boards'))->json();
    }

    /** @param  array<string, mixed>  $project */
    public function render(array $project): RenderResult
    {
        $res = $this->send(fn () => $this->request()->post('render', ['project' => $project]));

        return RenderResult::fromArray($res->json());
    }

    /** @param  array<string, mixed>  $project */
    public function export(array $project, string $format): ExportedFile
    {
        $res = $this->send(fn () => $this->request()
            ->withHeaders(['Accept' => '*/*'])
            ->post('export', ['project' => $project, 'format' => strtolower($format)]));

        return new ExportedFile(
            contents: $res->body(),
            mimeType: (string) ($res->header('Content-Type') ?: 'application/octet-stream'),
            filename: self::filenameFrom($res->header('Content-Disposition'), $project, $format),
            etag: $res->header('ETag') ?: null,
        );
    }

    protected function request(bool $auth = true): PendingRequest
    {
        $req = $this->http
            ->baseUrl(rtrim((string) ($this->config['url'] ?? 'http://localhost:8080/v1'), '/').'/')
            ->acceptJson()
            ->timeout((float) ($this->config['timeout'] ?? 15))
            ->retry(
                1 + max(0, (int) ($this->config['retries'] ?? 2)),
                max(0, (int) ($this->config['retry_delay_ms'] ?? 200)),
                fn (Throwable $e) => $e instanceof HttpConnectionException
                    || ($e instanceof RequestException
                        && ($e->response->serverError() || $e->response->status() === 429)),
                throw: false,
            );

        if ($auth) {
            $key = $this->config['key'] ?? null;
            if (! is_string($key) || $key === '') {
                throw new PackForgeException('PACKFORGE_KEY is not configured.', 0, 'missing_key');
            }
            $req = $req->withToken($key);
        }

        return $req;
    }

    /** @param  callable(): Response  $call */
    protected function send(callable $call): Response
    {
        try {
            $res = $call();
        } catch (HttpConnectionException $e) {
            throw new ConnectionException('Cannot reach the PackForge API: '.$e->getMessage(), 0, 'connection_failed', [], $e);
        }

        if ($res->failed()) {
            throw PackForgeException::fromResponse($res);
        }

        return $res;
    }

    /** @param  array<string, mixed>  $project */
    protected static function filenameFrom(?string $disposition, array $project, string $format): string
    {
        if ($disposition && preg_match('/filename="?([^";]+)"?/i', $disposition, $m)) {
            return basename($m[1]);
        }

        return ($project['component_id'] ?? 'design').'.'.strtolower($format);
    }
}
