<?php

namespace App\Http\Controllers;

use App\Jobs\GeneratePackagingFiles;
use App\Models\Order;
use Illuminate\Http\RedirectResponse;
use Illuminate\Http\Request;
use Illuminate\Support\Facades\Storage;
use Illuminate\Validation\ValidationException as LaravelValidation;
use Illuminate\View\View;
use PackForge\Laravel\Exceptions\PackForgeException;
use PackForge\Laravel\Exceptions\ValidationException;
use PackForge\Laravel\Facades\PackForge;
use Symfony\Component\HttpFoundation\StreamedResponse;

class OrderController extends Controller
{
    public function create(): View
    {
        return view('orders.create', [
            'project' => [
                'component_id' => 'fefco-0201',
                'params' => ['L' => 300, 'W' => 200, 'H' => 150],
                'board' => 'C-flute',
                'units' => 'mm',
            ],
        ]);
    }

    public function store(Request $request): RedirectResponse
    {
        $data = $request->validate([
            'customer_name' => ['required', 'string', 'max:120'],
            'customer_email' => ['required', 'email', 'max:200'],
            'quantity' => ['required', 'integer', 'min:1', 'max:100000'],
            'project' => ['required', 'json'],
        ]);
        $project = json_decode($data['project'], true);

        // Never trust the browser: re-render on the server to validate and get authoritative stats.
        try {
            $result = PackForge::design($project)->render();
        } catch (ValidationException $e) {
            throw LaravelValidation::withMessages(['project' => collect($e->messages())
                ->map(fn ($m, $p) => "{$p}: {$m}")->values()->all()]);
        } catch (PackForgeException $e) {
            report($e);

            return back()->withInput()->withErrors(['project' => 'The design service is unavailable, please try again.']);
        }

        $order = Order::create([
            'customer_name' => $data['customer_name'],
            'customer_email' => $data['customer_email'],
            'quantity' => $data['quantity'],
            'project' => $project,
            'stats' => $result->raw['stats'],
        ]);

        GeneratePackagingFiles::dispatch($order);

        return redirect()->route('orders.show', $order);
    }

    public function show(Order $order): View
    {
        return view('orders.show', ['order' => $order]);
    }

    public function download(Order $order, string $format): StreamedResponse
    {
        $path = match ($format) {
            'dxf' => $order->dxf_path,
            'svg' => $order->svg_path,
            default => null,
        };
        abort_unless($path && Storage::exists($path), 404);

        return Storage::download($path);
    }
}
