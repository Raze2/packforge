<?php

namespace PackForge\Laravel\Facades;

use Illuminate\Support\Facades\Facade;
use PackForge\Laravel\Client;

/**
 * @method static \PackForge\Laravel\Design component(string $componentId)
 * @method static \PackForge\Laravel\Design design(array $project)
 * @method static array health()
 * @method static array components()
 * @method static array describe(string $componentId)
 * @method static array boards()
 * @method static \PackForge\Laravel\RenderResult render(array $project)
 * @method static \PackForge\Laravel\ExportedFile export(array $project, string $format, array $options = [])
 * @method static array nest(array $project, array $nesting)
 * @method static array fold(array $project, float $progress = 1.0)
 * @method static array pallet(array $project, array $pallet = [])
 * @method static \PackForge\Laravel\ExportedFile palletReport(array $project, array $pallet = [])
 * @method static array caseOptimize(array $product, int $count, array $options = [])
 *
 * @see Client
 */
class PackForge extends Facade
{
    protected static function getFacadeAccessor(): string
    {
        return Client::class;
    }
}
