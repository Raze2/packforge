<?php

namespace PackForge\Laravel\Tests;

class DesignerComponentTest extends TestCase
{
    public function test_renders_the_web_component_with_escaped_attributes(): void
    {
        $html = $this->blade(
            '<x-packforge-designer component="fefco-0201" :params="$params" board="B-flute" mode="configurator" height="500px" class="my-designer" />',
            ['params' => ['L' => 400, 'note' => '"><script>alert(1)</script>']],
        );

        $html->assertSee('<script type="module" src="/vendor/packforge/packforge-designer.js"></script>', false);
        $html->assertSee('<packforge-designer', false);
        $html->assertSee('component="fefco-0201"', false);
        $html->assertSee('board="B-flute"', false);
        $html->assertSee('export-endpoint="/packforge/export"', false);
        $html->assertSee('csrf-token=', false);
        $html->assertSee('style="display:block;height:500px"', false);
        $html->assertSee('class="my-designer"', false);
        $html->assertDontSee('<script>alert(1)</script>', false);
        $html->assertSee('params="{&quot;L&quot;:400', false);
    }

    public function test_project_overrides_and_custom_board_json(): void
    {
        $project = [
            'component_id' => 'fefco-0201',
            'params' => ['L' => 250],
            'board' => ['id' => 'custom', 'caliper_mm' => 3.2],
            'units' => 'in',
        ];

        $html = $this->blade('<x-packforge-designer :project="$project" :export-endpoint="false" />', ['project' => $project]);

        $html->assertSee('units="in"', false);
        $html->assertSee('board="{&quot;id&quot;:&quot;custom&quot;,&quot;caliper_mm&quot;:3.2}"', false);
        $html->assertDontSee('export-endpoint=', false);
    }

    public function test_locale_follows_the_app_locale(): void
    {
        app()->setLocale('ar');

        $this->blade('<x-packforge-designer />')->assertSee('locale="ar"', false);

        app()->setLocale('de');

        $this->blade('<x-packforge-designer />')->assertSee('locale="en"', false);
    }
}
