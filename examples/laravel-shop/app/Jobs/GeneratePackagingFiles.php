<?php

namespace App\Jobs;

use App\Models\Order;
use Illuminate\Contracts\Queue\ShouldQueue;
use Illuminate\Foundation\Queue\Queueable;
use PackForge\Laravel\Exceptions\RateLimitedException;
use PackForge\Laravel\Exceptions\ValidationException;
use PackForge\Laravel\Facades\PackForge;
use Throwable;

/**
 * Generates production files (DXF for the cutting table, SVG for proofs) for an order.
 */
class GeneratePackagingFiles implements ShouldQueue
{
    use Queueable;

    public int $tries = 5;

    /** @var array<int, int> */
    public array $backoff = [5, 30, 120];

    public function __construct(public Order $order) {}

    public function handle(): void
    {
        $design = PackForge::design($this->order->project);
        $dir = "orders/{$this->order->id}";

        try {
            $dxf = $design->dxf();
            $svg = $design->svg();
            // 1:1 PDF proof for the customer's approval, stamped so it is not used as artwork.
            $pdf = $design->pdf(['watermark' => 'Proof - not for production']);
        } catch (RateLimitedException $e) {
            $this->release($e->retryAfter);

            return;
        } catch (ValidationException $e) {
            // The design itself is invalid: retrying will not help.
            $this->order->update(['status' => 'failed', 'error' => $e->getMessage()]);
            $this->fail($e);

            return;
        }

        $this->order->update([
            'status' => 'ready',
            'dxf_path' => $dxf->store("{$dir}/{$dxf->filename}"),
            'svg_path' => $svg->store("{$dir}/{$svg->filename}"),
            'pdf_path' => $pdf->store("{$dir}/{$pdf->filename}"),
            'error' => null,
        ]);
    }

    public function failed(?Throwable $e): void
    {
        $this->order->update(['status' => 'failed', 'error' => $e?->getMessage()]);
    }
}
