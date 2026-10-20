# Box Shop — PackForge Laravel example

A small Laravel 13 shop where customers design a box, place an order, and a queued job
generates the production files (DXF for the cutting table, SVG proof) through the PackForge API.

| Part | File |
|---|---|
| Configurator page with `<x-packforge-designer>` | `resources/views/orders/create.blade.php` |
| Server-side validation + order creation | `app/Http/Controllers/OrderController.php` |
| Queued file generation | `app/Jobs/GeneratePackagingFiles.php` |
| Order status page and downloads | `resources/views/orders/show.blade.php` |
| End-to-end test (API faked) | `tests/Feature/OrderFlowTest.php` |

The package is installed from `../../integrations/laravel` through a Composer path repository.

## Run locally

```bash
# 1. API (from the repository root)
cargo run -p pf-api --release -- gen-key --id local-dev --prefix pk_test > out/keys.gen.txt
grep -v '^#' out/keys.gen.txt > out/keys.toml          # the secret is the 2nd line of keys.gen.txt
PF_KEYS_FILE=out/keys.toml PF_BIND=127.0.0.1:8080 cargo run -p pf-api --release

# 2. Shop (from this folder)
composer install
cp .env.example .env && php artisan key:generate       # then set PACKFORGE_KEY to the secret
php artisan migrate
php artisan vendor:publish --tag=packforge-assets
php artisan serve --port=8000                          # http://localhost:8000
php artisan queue:work                                 # separate terminal
```

## Test

```bash
php artisan test
```

The demo has no authentication; a real shop would put the configurator, orders and the
`packforge/export` route behind its own auth and authorization.
