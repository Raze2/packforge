@extends('layout')

@section('title', 'Design your box')

@section('content')
    <h1>Design your shipping box</h1>
    <p class="sub">Set the inside dimensions; the die-line updates live. Production files are generated after you order.</p>

    @if ($errors->any())
        <div class="errors">@foreach ($errors->all() as $e)<div>{{ $e }}</div>@endforeach</div>
    @endif

    <div class="grid">
        <x-packforge-designer id="designer" :project="old('project') ? json_decode(old('project'), true) : $project" theme="light" height="680px" />

        <form class="card" method="post" action="{{ route('orders.store') }}" id="order-form">
            @csrf
            <h2 style="margin:0 0 4px;font-size:17px">Your order</h2>
            <p class="sub" style="margin:0" id="summary">Loading designer…</p>

            <label for="customer_name">Name</label>
            <input id="customer_name" name="customer_name" value="{{ old('customer_name') }}" required autocomplete="name">

            <label for="customer_email">Email</label>
            <input id="customer_email" name="customer_email" type="email" value="{{ old('customer_email') }}" required autocomplete="email">

            <label for="quantity">Quantity</label>
            <input id="quantity" name="quantity" type="number" min="1" value="{{ old('quantity', 250) }}" required>

            <input type="hidden" name="project" id="project" value="{{ old('project', json_encode($project)) }}">

            <div class="actions"><button type="submit" id="submit">Place order</button></div>
        </form>
    </div>

    <script type="module">
        const designer = document.getElementById('designer');
        const field = document.getElementById('project');
        const summary = document.getElementById('summary');
        const submit = document.getElementById('submit');
        designer.addEventListener('pf:change', (e) => {
            const { project, result, error } = e.detail;
            field.value = JSON.stringify(project);
            submit.disabled = !!error;
            summary.textContent = error
                ? error.message
                : `Blank ${Math.round(result.stats.blank_width)} × ${Math.round(result.stats.blank_height)} mm · ${result.stats.blank_area_m2.toFixed(3)} m² per box`;
        });
        // Saving from the designer toolbar keeps the design in the form instead of downloading it.
        designer.addEventListener('pf:save', (e) => { e.preventDefault(); field.value = JSON.stringify(e.detail.project); });
    </script>
@endsection
