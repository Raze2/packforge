<?php

use Illuminate\Database\Migrations\Migration;
use Illuminate\Database\Schema\Blueprint;
use Illuminate\Support\Facades\Schema;

return new class extends Migration
{
    public function up(): void
    {
        Schema::create('orders', function (Blueprint $table) {
            $table->id();
            $table->string('customer_name');
            $table->string('customer_email');
            $table->unsignedInteger('quantity');
            $table->json('project');            // PackForge project (component, params, board)
            $table->json('stats')->nullable();  // blank size, area, line lengths
            $table->string('status')->default('pending'); // pending | ready | failed
            $table->string('dxf_path')->nullable();
            $table->string('svg_path')->nullable();
            $table->text('error')->nullable();
            $table->timestamps();
        });
    }

    public function down(): void
    {
        Schema::dropIfExists('orders');
    }
};
