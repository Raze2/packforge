<?php

namespace PackForge\Laravel;

use InvalidArgumentException;
use JsonSerializable;

/**
 * Fluent, immutable-style builder for a PackForge project. Lengths are millimetres.
 */
class Design implements JsonSerializable
{
    /** @var array<string, int|float|string|bool> */
    protected array $params = [];

    /** @var string|array{id: string, name?: string, caliper_mm: float} */
    protected string|array $board = 'C-flute';

    protected string $units = 'mm';

    protected ?string $componentVersion = null;

    public function __construct(
        protected Client $client,
        protected string $componentId,
    ) {}

    /** @param  array<string, mixed>  $project */
    public static function fromArray(Client $client, array $project): self
    {
        if (! isset($project['component_id']) || ! is_string($project['component_id'])) {
            throw new InvalidArgumentException('Project is missing component_id.');
        }
        $d = new self($client, $project['component_id']);
        $d->params = (array) ($project['params'] ?? []);
        $d->board = $project['board'] ?? 'C-flute';
        $d->units = (string) ($project['units'] ?? 'mm');
        $d->componentVersion = $project['component_version'] ?? null;

        return $d;
    }

    /** Merges parameters, e.g. ['L' => 300, 'W' => 200, 'H' => 150]. */
    public function params(array $params): static
    {
        $clone = clone $this;
        $clone->params = array_merge($this->params, $params);

        return $clone;
    }

    public function param(string $id, int|float|string|bool $value): static
    {
        return $this->params([$id => $value]);
    }

    /** Uses a board preset, e.g. 'C-flute' (see PackForge::boards()). */
    public function board(string $presetId): static
    {
        $clone = clone $this;
        $clone->board = $presetId;

        return $clone;
    }

    /** Uses a custom board with a measured caliper in millimetres. */
    public function caliper(float $mm, string $id = 'custom', string $name = 'Custom'): static
    {
        if ($mm <= 0) {
            throw new InvalidArgumentException('Caliper must be greater than zero.');
        }
        $clone = clone $this;
        $clone->board = ['id' => $id, 'name' => $name, 'caliper_mm' => $mm];

        return $clone;
    }

    /** Display units stored with the project (mm, cm, in). Params stay in mm. */
    public function units(string $units): static
    {
        if (! in_array($units, ['mm', 'cm', 'in'], true)) {
            throw new InvalidArgumentException("Unsupported units '{$units}'.");
        }
        $clone = clone $this;
        $clone->units = $units;

        return $clone;
    }

    public function render(): RenderResult
    {
        return $this->client->render($this->toArray());
    }

    /**
     * @param  array<string, mixed>  $options  rotate (0/90/180/270), mirror, page (actual/a4/a3/letter),
     *                                         watermark, profile (standard/cut-crease), nesting
     */
    public function export(string $format, array $options = []): ExportedFile
    {
        return $this->client->export($this->toArray(), $format, $options);
    }

    public function dxf(array $options = []): ExportedFile
    {
        return $this->export('dxf', $options);
    }

    public function svg(array $options = []): ExportedFile
    {
        return $this->export('svg', $options);
    }

    /** PDF at 1:1 by default; pass ['page' => 'a4'] to fit a sheet. */
    public function pdf(array $options = []): ExportedFile
    {
        return $this->export('pdf', $options);
    }

    /** CF2 for die-makers and cutting tables. */
    public function cf2(array $options = []): ExportedFile
    {
        return $this->export('cf2', $options);
    }

    /** HPGL plot file for plotters and sample tables. */
    public function plt(array $options = []): ExportedFile
    {
        return $this->export('hpgl', $options);
    }

    /** Binary glTF 3D model of the erected box. */
    public function glb(): ExportedFile
    {
        return $this->export('glb');
    }

    /**
     * How many blanks fit on a sheet (mm): e.g. ['sheet_width' => 2400, 'sheet_height' => 1600].
     *
     * @return array<string, mixed>
     */
    public function nest(array $nesting): array
    {
        return $this->client->nest($this->toArray(), $nesting);
    }

    /**
     * Pallet plan, e.g. ['pallet' => 'eur', 'max_height' => 1800, 'box_weight' => 8].
     *
     * @return array<string, mixed>
     */
    public function pallet(array $pallet = []): array
    {
        return $this->client->pallet($this->toArray(), $pallet);
    }

    public function palletReport(array $pallet = []): ExportedFile
    {
        return $this->client->palletReport($this->toArray(), $pallet);
    }

    /** @return array<string, mixed> */
    public function fold(float $progress = 1.0): array
    {
        return $this->client->fold($this->toArray(), $progress);
    }

    /** @return array<string, mixed> project payload as sent to the API */
    public function toArray(): array
    {
        return array_filter([
            'component_id' => $this->componentId,
            'component_version' => $this->componentVersion,
            'params' => (object) $this->params,
            'board' => $this->board,
            'units' => $this->units,
        ], fn ($v) => $v !== null);
    }

    public function jsonSerialize(): array
    {
        return $this->toArray();
    }
}
