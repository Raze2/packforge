<?php

namespace PackForge\Laravel\Exceptions;

/** One or more design parameters are invalid (HTTP 400, code invalid_params). */
class ValidationException extends PackForgeException
{
    /**
     * Errors keyed by parameter id, e.g. ['L' => 'must be at least 40'].
     *
     * @return array<string, string>
     */
    public function messages(): array
    {
        $out = [];
        foreach ($this->errors as $e) {
            $out[$e['param']] = $e['message'];
        }

        return $out;
    }
}
