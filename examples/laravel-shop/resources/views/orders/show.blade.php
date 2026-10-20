@extends('layout', ['refresh' => $order->status === 'pending' ? 2 : null])

@section('title', 'Order #'.$order->id)

@section('content')
    <h1>Order #{{ $order->id }} <span class="status {{ $order->status }}">{{ ucfirst($order->status) }}</span></h1>
    <p class="sub">
        @switch($order->status)
            @case('pending') Generating production files… this page refreshes automatically. @break
            @case('ready') Production files are ready. @break
            @default Something went wrong: {{ $order->error }}
        @endswitch
    </p>

    <div class="grid">
        <div class="card">
            @if ($order->status === 'ready')
                <img src="{{ route('orders.download', [$order, 'svg']) }}" alt="Die-line preview" style="width:100%;height:auto;background:#fbfbfc;border-radius:10px">
            @else
                <x-packforge-designer :project="$order->project" mode="configurator" theme="light" :export-endpoint="false" height="520px" />
            @endif
        </div>

        <div class="card">
            <dl>
                <dt>Customer</dt><dd>{{ $order->customer_name }}</dd>
                <dt>Style</dt><dd>{{ $order->project['component_id'] }}</dd>
                <dt>Inside L×W×H</dt>
                <dd>{{ $order->project['params']['L'] ?? '—' }} × {{ $order->project['params']['W'] ?? '—' }} × {{ $order->project['params']['H'] ?? '—' }} mm</dd>
                <dt>Board</dt><dd>{{ is_array($order->project['board']) ? ($order->project['board']['caliper_mm'].' mm custom') : $order->project['board'] }}</dd>
                <dt>Quantity</dt><dd>{{ number_format($order->quantity) }}</dd>
                @if ($order->stats)
                    <dt>Blank</dt><dd>{{ round($order->stats['blank_width']) }} × {{ round($order->stats['blank_height']) }} mm</dd>
                    <dt>Board total</dt><dd>{{ number_format($order->stats['blank_area_m2'] * $order->quantity, 1) }} m²</dd>
                    <dt>Board cost</dt><dd>{{ number_format($order->estimatedBoardCost(), 2) }} {{ config('shop.currency') }}</dd>
                @endif
            </dl>
            @if ($order->status === 'ready')
                <div class="actions">
                    <a class="btn" href="{{ route('orders.download', [$order, 'dxf']) }}">Download DXF</a>
                    <a class="btn ghost" href="{{ route('orders.download', [$order, 'svg']) }}">Download SVG</a>
                </div>
            @endif
        </div>
    </div>
@endsection
