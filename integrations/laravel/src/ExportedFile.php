<?php

namespace PackForge\Laravel;

use Illuminate\Contracts\Support\Responsable;
use Illuminate\Support\Facades\Storage;
use Symfony\Component\HttpFoundation\Response;

/** An exported design file (DXF, SVG…). Return it from a controller to download it. */
final class ExportedFile implements Responsable
{
    public function __construct(
        public readonly string $contents,
        public readonly string $mimeType,
        public readonly string $filename,
        public readonly ?string $etag = null,
    ) {}

    public function contents(): string
    {
        return $this->contents;
    }

    public function size(): int
    {
        return strlen($this->contents);
    }

    /** Stores the file on a filesystem disk; returns the path. */
    public function store(string $path, ?string $disk = null): string
    {
        Storage::disk($disk)->put($path, $this->contents);

        return $path;
    }

    public function toResponse($request): Response
    {
        $headers = [
            'Content-Type' => $this->mimeType,
            'Content-Disposition' => 'attachment; filename="'.addcslashes($this->filename, '"\\').'"',
            'Content-Length' => (string) $this->size(),
        ];
        if ($this->etag) {
            $headers['ETag'] = $this->etag;
        }

        return new Response($this->contents, 200, $headers);
    }
}
