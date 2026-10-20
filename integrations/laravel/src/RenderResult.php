<?php

namespace PackForge\Laravel;

/** Result of POST /v1/render. `raw` holds the full response, including die-line geometry. */
final class RenderResult
{
    /**
     * @param  array<int, array{severity: string, message: string}>  $diagnostics
     * @param  array<string, mixed>  $raw
     */
    public function __construct(
        public readonly string $componentId,
        public readonly string $componentVersion,
        public readonly Stats $stats,
        public readonly array $diagnostics,
        public readonly array $raw,
    ) {}

    /** @param  array<string, mixed>  $r */
    public static function fromArray(array $r): self
    {
        return new self(
            (string) ($r['component_id'] ?? ''),
            (string) ($r['component_version'] ?? ''),
            Stats::fromArray((array) ($r['stats'] ?? [])),
            array_values((array) ($r['diagnostics'] ?? [])),
            $r,
        );
    }

    /** @return array<int, string> */
    public function warnings(): array
    {
        return array_values(array_map(
            fn ($d) => $d['message'],
            array_filter($this->diagnostics, fn ($d) => ($d['severity'] ?? '') === 'warning'),
        ));
    }
}
