<?php

namespace PackForge\Laravel;

use Illuminate\Http\Client\Factory as HttpFactory;
use Illuminate\Support\Facades\Blade;
use Illuminate\Support\Facades\Route;
use Illuminate\Support\ServiceProvider;
use PackForge\Laravel\Http\Controllers\ExportController;
use PackForge\Laravel\View\Components\Designer;

class PackForgeServiceProvider extends ServiceProvider
{
    public function register(): void
    {
        $this->mergeConfigFrom(__DIR__.'/../config/packforge.php', 'packforge');

        $this->app->singleton(Client::class, fn ($app) => new Client(
            $app->make(HttpFactory::class),
            (array) $app['config']->get('packforge', []),
        ));
    }

    public function boot(): void
    {
        $this->loadViewsFrom(__DIR__.'/../resources/views', 'packforge');
        Blade::component('packforge-designer', Designer::class);

        if ($this->app['config']->get('packforge.export_route.enabled', true)) {
            Route::middleware((array) $this->app['config']->get('packforge.export_route.middleware', ['web']))
                ->post(
                    (string) $this->app['config']->get('packforge.export_route.uri', 'packforge/export'),
                    ExportController::class,
                )
                ->name((string) $this->app['config']->get('packforge.export_route.name', 'packforge.export'));
        }

        if ($this->app->runningInConsole()) {
            $this->publishes([
                __DIR__.'/../config/packforge.php' => config_path('packforge.php'),
            ], 'packforge-config');

            $this->publishes([
                __DIR__.'/../resources/dist/packforge-designer.js' => public_path('vendor/packforge/packforge-designer.js'),
            ], 'packforge-assets');
        }
    }
}
