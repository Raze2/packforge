<?php

namespace App\Models;

use Illuminate\Database\Eloquent\Model;

class Order extends Model
{
    protected $fillable = [
        'customer_name', 'customer_email', 'quantity', 'project', 'stats',
        'status', 'dxf_path', 'svg_path', 'pdf_path', 'error',
    ];

    protected function casts(): array
    {
        return [
            'project' => 'array',
            'stats' => 'array',
            'quantity' => 'integer',
        ];
    }

    /** Board cost estimate: blank area × quantity × price per m². */
    public function estimatedBoardCost(): ?float
    {
        $area = $this->stats['blank_area_m2'] ?? null;

        return $area === null ? null : round($area * $this->quantity * (float) config('shop.board_price_per_m2'), 2);
    }
}
