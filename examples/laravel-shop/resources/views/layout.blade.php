<!doctype html>
<html lang="{{ str_replace('_', '-', app()->getLocale()) }}">
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1">
    <meta name="csrf-token" content="{{ csrf_token() }}">
    @isset($refresh)<meta http-equiv="refresh" content="{{ $refresh }}">@endisset
    <title>@yield('title') · Box Shop</title>
    <style>
        :root { --bg:#f6f7f9; --card:#fff; --text:#16181d; --muted:#667085; --line:#e3e6ea; --accent:#3a5bd9; --ok:#16794a; --bad:#b42318; }
        * { box-sizing: border-box; }
        body { margin:0; font:15px/1.5 system-ui,-apple-system,'Segoe UI',Roboto,sans-serif; background:var(--bg); color:var(--text); }
        header { background:var(--card); border-bottom:1px solid var(--line); padding:14px 16px; }
        header a { color:inherit; text-decoration:none; font-weight:700; }
        main { max-width:1280px; margin:0 auto; padding:20px 16px 48px; }
        h1 { font-size:22px; margin:0 0 4px; }
        .sub { color:var(--muted); margin:0 0 18px; }
        .grid { display:grid; grid-template-columns:minmax(0,1fr) 320px; gap:16px; align-items:start; }
        .card { background:var(--card); border:1px solid var(--line); border-radius:14px; padding:18px; }
        label { display:block; font-weight:600; font-size:13px; margin:12px 0 6px; }
        input { width:100%; height:38px; padding:0 10px; border:1px solid var(--line); border-radius:8px; font:inherit; }
        button, .btn { display:inline-flex; align-items:center; justify-content:center; height:40px; padding:0 18px; border:0; border-radius:9px; background:var(--accent); color:#fff; font:inherit; font-weight:600; cursor:pointer; text-decoration:none; }
        .btn.ghost { background:transparent; color:var(--accent); border:1px solid var(--accent); }
        .errors { background:#fef3f2; color:var(--bad); border-radius:10px; padding:10px 14px; margin-bottom:14px; }
        dl { display:grid; grid-template-columns:auto 1fr; gap:6px 16px; margin:0; }
        dt { color:var(--muted); } dd { margin:0; font-weight:600; font-variant-numeric:tabular-nums; }
        .status { display:inline-block; padding:2px 10px; border-radius:99px; font-size:13px; font-weight:600; }
        .status.pending { background:#fff7e6; color:#8a5a00; } .status.ready { background:#ecfdf3; color:var(--ok); } .status.failed { background:#fef3f2; color:var(--bad); }
        .actions { display:flex; gap:10px; flex-wrap:wrap; margin-top:16px; }
        @media (max-width: 900px) { .grid { grid-template-columns:1fr; } }
    </style>
</head>
<body>
<header><a href="{{ route('orders.create') }}">📦 Box Shop</a> <span style="color:var(--muted)">— PackForge example</span></header>
<main>@yield('content')</main>
</body>
</html>
