<?php

namespace PackForge\Laravel\View\Components;

use Closure;
use Illuminate\Contracts\View\View;
use Illuminate\Support\Facades\Route;
use Illuminate\View\Component;

/**
 * <x-packforge-designer :project="$design->project" mode="configurator" />
 *
 * Renders the <packforge-designer> web component. Live preview runs in the browser (WASM);
 * exports go through the package's export route so the API key stays on the server.
 */
class Designer extends Component
{
    public string $componentId;

    /** @var array<string, mixed> */
    public array $params;

    public string $boardAttr;

    public ?string $exportUrl;

    public string $scriptUrl;

    public string $locale;

    /**
     * @param  array<string, mixed>|null  $project  saved project (overrides component/params/board)
     * @param  array<string, mixed>  $params
     * @param  string|array<string, mixed>  $board
     * @param  string|false|null  $exportEndpoint  null = package route, false = browser-only exports
     */
    public function __construct(
        ?array $project = null,
        string $component = 'fefco-0201',
        array $params = [],
        string|array $board = 'C-flute',
        public string $mode = 'configurator',
        ?string $locale = null,
        public string $theme = 'auto',
        public string $units = 'mm',
        string|false|null $exportEndpoint = null,
        public bool $persist = false,
        public string $height = '640px',
    ) {
        $this->componentId = (string) ($project['component_id'] ?? $component);
        $this->params = (array) ($project['params'] ?? $params);
        $board = $project['board'] ?? $board;
        $this->boardAttr = is_array($board) ? (string) json_encode($board) : $board;
        $this->units = (string) ($project['units'] ?? $units);

        $appLocale = substr((string) app()->getLocale(), 0, 2);
        $this->locale = $locale ?? (in_array($appLocale, ['en', 'fr', 'ar'], true) ? $appLocale : 'en');

        $routeName = (string) config('packforge.export_route.name', 'packforge.export');
        $this->exportUrl = match (true) {
            $exportEndpoint === false => null,
            is_string($exportEndpoint) => $exportEndpoint,
            config('packforge.export_route.enabled', true) && Route::has($routeName) => route($routeName, [], false),
            default => null,
        };

        $this->scriptUrl = (string) config('packforge.designer_script', '/vendor/packforge/packforge-designer.js');
    }

    public function render(): View|Closure|string
    {
        return view('packforge::components.designer');
    }
}
