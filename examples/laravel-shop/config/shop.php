<?php

return [
    // Example board price used for the cost estimate on the order page.
    'board_price_per_m2' => (float) env('SHOP_BOARD_PRICE_PER_M2', 0.65),
    'currency' => env('SHOP_CURRENCY', 'EUR'),
];
