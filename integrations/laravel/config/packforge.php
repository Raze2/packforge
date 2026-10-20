<?php

return [

    /*
    | Base URL of the PackForge API (pf-api), including the /v1 prefix.
    */
    'url' => env('PACKFORGE_URL', 'http://localhost:8080/v1'),

    /*
    | Secret API key (pk_live_… / pk_test_…). Server-side only: never expose it to browsers.
    */
    'key' => env('PACKFORGE_KEY'),

    /*
    | HTTP behaviour. Retries apply to connection errors and 5xx/429 responses.
    */
    'timeout' => (float) env('PACKFORGE_TIMEOUT', 15),
    'retries' => (int) env('PACKFORGE_RETRIES', 2),
    'retry_delay_ms' => (int) env('PACKFORGE_RETRY_DELAY_MS', 200),

    /*
    | Browser exports from <x-packforge-designer> are posted to this route, which forwards them
    | to the API with the secret key. Protect it with your own middleware (auth, throttle…).
    */
    'export_route' => [
        'enabled' => (bool) env('PACKFORGE_EXPORT_ROUTE', true),
        'uri' => 'packforge/export',
        'name' => 'packforge.export',
        'middleware' => ['web', 'throttle:60,1'],
        'formats' => ['svg', 'dxf'],
    ],

    /*
    | URL of the <packforge-designer> script. Publish it with:
    |   php artisan vendor:publish --tag=packforge-assets
    */
    'designer_script' => env('PACKFORGE_DESIGNER_SCRIPT', '/vendor/packforge/packforge-designer.js'),

];
