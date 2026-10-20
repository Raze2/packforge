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

    /**
     * Exports a file: svg, dxf, pdf, cf2, hpgl (plt) or glb.
     *
     * @param  array<string, mixed>  $project
     * @param  array<string, mixed>  $options  rotate, mirror, page, watermark, profile, nesting
     */
    public function export(array $project, string $format, array $options = []): ExportedFile
    {
        $body = ['project' => $project, 'format' => strtolower($format)];
        if ($options !== []) {
            $body['options'] = $options;
        }
        $res = $this->send(fn () => $this->request()
            ->withHeaders(['Accept' => '*/*'])
            ->post('export', $body));

        return new ExportedFile(
            contents: $res->body(),
            mimeType: (string) ($res->header('Content-Type') ?: 'application/octet-stream'),
            filename: self::filenameFrom($res->header('Content-Disposition'), $project, $format),
            etag: $res->header('ETag') ?: null,
        );
    }

    /**
     * How many blanks fit on a sheet.
     *
     * @param  array<string, mixed>  $project
     * @param  array{sheet_width: float|int, sheet_height: float|int, gap?: float|int, margin?: float|int, allow_rotation?: bool, max_count?: int}  $nesting
     * @return array{count: int, cols: int, rows: int, rotated: bool, blank_width: float, blank_height: float, sheet_area_m2: float, utilisation: float}
     */
    public function nest(array $project, array $nesting): array
    {
        return $this->send(fn () => $this->request()->post('nest', ['project' => $project, 'nesting' => $nesting]))->json();
    }

    /**
     * 3D panel positions of the folded design (progress 0 = flat, 1 = erected).
     *
     * @param  array<string, mixed>  $project
     * @return array<string, mixed>
     */
    public function fold(array $project, float $progress = 1.0): array
    {
        return $this->send(fn () => $this->request()->post('fold', ['project' => $project, 'progress' => $progress]))->json();
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
