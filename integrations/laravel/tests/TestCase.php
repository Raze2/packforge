<?php

namespace PackForge\Laravel\Tests;

use Orchestra\Testbench\TestCase as Orchestra;
use PackForge\Laravel\PackForgeServiceProvider;

abstract class TestCase extends Orchestra
{
    protected function getPackageProviders($app): array
    {
        return [PackForgeServiceProvider::class];
    }

    protected function getPackageAliases($app): array
    {
        return ['PackForge' => \PackForge\Laravel\Facades\PackForge::class];
    }

    protected function defineEnvironment($app): void
    {
        $app['config']->set('packforge.url', 'https://api.packforge.test/v1');
        $app['config']->set('packforge.key', 'pk_test_123');
        $app['config']->set('packforge.retry_delay_ms', 0);
        $app['config']->set('app.key', 'base64:'.base64_encode(str_repeat('k', 32)));
    }

    /** @return array<string, mixed> */
    protected static function renderResponse(): array
    {
        return [
            'component_id' => 'fefco-0201',
            'component_version' => '0.1.0',
            'board' => ['id' => 'C-flute', 'name' => 'Single wall C flute', 'caliper_mm' => 4.0],
            'dieline' => ['entities' => []],
            'fold' => ['panels' => [], 'hinges' => [], 'thickness' => 4.0],
            'stats' => [
                'blank_width' => 1051.0, 'blank_height' => 362.0, 'blank_area_m2' => 0.380462,
                'outline_area_m2' => 0.3679, 'cut_length' => 4026.0, 'crease_length' => 2616.0,
            ],
            'diagnostics' => [['severity' => 'warning', 'message' => 'Width is larger than length.']],
        ];
    }
}
