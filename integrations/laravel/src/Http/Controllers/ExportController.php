<?php

namespace PackForge\Laravel\Http\Controllers;

use Illuminate\Http\JsonResponse;
use Illuminate\Http\Request;
use Illuminate\Routing\Controller;
use PackForge\Laravel\Client;
use PackForge\Laravel\Exceptions\ConnectionException;
use PackForge\Laravel\Exceptions\PackForgeException;
use PackForge\Laravel\Exceptions\ValidationException;
use PackForge\Laravel\ExportedFile;

/**
 * Proxies browser exports from <x-packforge-designer> to the API, adding the secret key.
 * Body: {"project": {...}, "format": "dxf"}.
 */
class ExportController extends Controller
{
    public function __invoke(Request $request, Client $client): ExportedFile|JsonResponse
    {
        $formats = (array) config('packforge.export_route.formats', ['svg', 'dxf']);
        $data = $request->validate([
            'project' => ['required', 'array'],
            'project.component_id' => ['required', 'string', 'max:100'],
            'project.params' => ['sometimes', 'array'],
            'project.board' => ['required'],
            'format' => ['required', 'string', 'in:'.implode(',', $formats)],
        ]);

        try {
            return $client->design($data['project'])->export($data['format']);
        } catch (ValidationException $e) {
            return response()->json(['message' => $e->getMessage(), 'errors' => $e->messages()], 422);
        } catch (ConnectionException $e) {
            report($e);

            return response()->json(['message' => 'The design service is unavailable.'], 503);
        } catch (PackForgeException $e) {
            report($e);
            $status = in_array($e->status, [400, 404, 422], true) ? 422 : 502;

            return response()->json(['message' => $e->getMessage(), 'code' => $e->problemCode], $status);
        }
    }
}
