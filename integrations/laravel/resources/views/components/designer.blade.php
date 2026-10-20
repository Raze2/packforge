@once
    <script type="module" src="{{ $scriptUrl }}"></script>
@endonce
<packforge-designer
    {{ $attributes->merge(['style' => "display:block;height:{$height}"]) }}
    component="{{ $componentId }}"
    params="{{ json_encode((object) $params) }}"
    board="{{ $boardAttr }}"
    mode="{{ $mode }}"
    locale="{{ $locale }}"
    theme="{{ $theme }}"
    units="{{ $units }}"
    @if ($exportUrl) export-endpoint="{{ $exportUrl }}" csrf-token="{{ csrf_token() }}" @endif
    @if ($persist) persist @endif
></packforge-designer>
