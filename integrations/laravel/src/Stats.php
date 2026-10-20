<?php

namespace PackForge\Laravel;

/** Blank size (mm), areas (m²) and line lengths (mm) of a rendered design. */
final class Stats
{
    public function __construct(
        public readonly float $blankWidth,
        public readonly float $blankHeight,
        public readonly float $blankAreaM2,
        public readonly float $outlineAreaM2,
        public readonly float $cutLength,
        public readonly float $creaseLength,
    ) {}

    /** @param  array<string, mixed>  $s */
    public static function fromArray(array $s): self
    {
        return new self(
            (float) ($s['blank_width'] ?? 0),
            (float) ($s['blank_height'] ?? 0),
            (float) ($s['blank_area_m2'] ?? 0),
            (float) ($s['outline_area_m2'] ?? 0),
            (float) ($s['cut_length'] ?? 0),
            (float) ($s['crease_length'] ?? 0),
        );
    }
}
