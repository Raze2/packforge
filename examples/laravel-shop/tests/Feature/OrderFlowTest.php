<?php

namespace Tests\Feature;

use App\Models\Order;
use Illuminate\Foundation\Testing\RefreshDatabase;
use Illuminate\Support\Facades\Http;
use Illuminate\Support\Facades\Storage;
use Tests\TestCase;

class OrderFlowTest extends TestCase
{
    use RefreshDatabase;

    private function project(array $params = ['L' => 300, 'W' => 200, 'H' => 150]): string
    {
        return json_encode(['component_id' => 'fefco-0201', 'params' => $params, 'board' => 'C-flute', 'units' => 'mm']);
    }

    private function fakeApi(): void
    {
        config(['packforge.key' => 'pk_test_123', 'packforge.retry_delay_ms' => 0]);
        Http::fake([
            '*/render' => Http::response([
                'component_id' => 'fefco-0201', 'component_version' => '0.1.0',
                'stats' => ['blank_width' => 1051.0, 'blank_height' => 362.0, 'blank_area_m2' => 0.380462,
                    'outline_area_m2' => 0.3679, 'cut_length' => 4026.0, 'crease_length' => 2616.0],
                'diagnostics' => [],
            ]),
            '*/export' => fn ($request) => Http::response(
                $request['format'] === 'dxf' ? "  0\nEOF\n" : '<svg/>',
                200,
                [
                    'Content-Type' => $request['format'] === 'dxf' ? 'application/dxf' : 'image/svg+xml',
                    'Content-Disposition' => 'attachment; filename="fefco-0201.'.$request['format'].'"',
                ],
            ),
        ]);
    }

    public function test_configurator_page_embeds_the_designer(): void
    {
        $this->get('/')
            ->assertOk()
            ->assertSee('<packforge-designer', false)
            ->assertSee('export-endpoint="/packforge/export"', false)
            ->assertSee('name="project"', false);
    }

    public function test_placing_an_order_generates_production_files(): void
    {
        Storage::fake('local');
        $this->fakeApi();

        $response = $this->post('/orders', [
            'customer_name' => 'Ada', 'customer_email' => 'ada@example.com',
            'quantity' => 250, 'project' => $this->project(),
        ]);

        $order = Order::sole();
        $response->assertRedirect(route('orders.show', $order));

        // The queue runs synchronously in tests, so files exist already.
        $order->refresh();
        $this->assertSame('ready', $order->status);
        Storage::disk('local')->assertExists("orders/{$order->id}/fefco-0201.dxf");
        Storage::disk('local')->assertExists("orders/{$order->id}/fefco-0201.svg");
        $this->assertEquals(1051, $order->stats['blank_width']);
        $this->assertSame(61.83, $order->estimatedBoardCost()); // 0.380462 m² × 250 × 0.65

        $this->get(route('orders.show', $order))->assertOk()->assertSee('Download DXF');
        $this->get(route('orders.download', [$order, 'dxf']))->assertOk()->assertDownload('fefco-0201.dxf');
    }

    public function test_invalid_designs_are_rejected_with_messages(): void
    {
        config(['packforge.key' => 'pk_test_123']);
        Http::fake(['*' => Http::response([
            'code' => 'invalid_params', 'detail' => 'invalid parameters',
            'errors' => [['param' => 'L', 'message' => 'must be at least 40']],
        ], 400)]);

        $this->from('/')->post('/orders', [
            'customer_name' => 'Ada', 'customer_email' => 'ada@example.com',
            'quantity' => 10, 'project' => $this->project(['L' => 5]),
        ])->assertRedirect('/')->assertSessionHasErrors(['project']);

        $this->assertSame(0, Order::count());
    }
}
