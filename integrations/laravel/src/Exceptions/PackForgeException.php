<?php

namespace PackForge\Laravel\Exceptions;

use Illuminate\Http\Client\Response;
use RuntimeException;
use Throwable;

/**
 * Base exception for PackForge API errors. Carries the RFC 7807 problem details.
 */
class PackForgeException extends RuntimeException
{
    /**
     * @param  array<int, array{param: string, message: string}>  $errors
     */
    public function __construct(
        string $message,
        public readonly int $status = 0,
        public readonly string $problemCode = 'unknown',
        public readonly array $errors = [],
        ?Throwable $previous = null,
    ) {
        parent::__construct($message, $status, $previous);
    }

    /**
     * Builds the most specific exception for a failed API response.
     */
    public static function fromResponse(Response $response): self
    {
        $problem = $response->json();
        $problem = is_array($problem) ? $problem : [];
        $status = $response->status();
        $code = (string) ($problem['code'] ?? 'http_'.$status);
        $message = (string) ($problem['detail'] ?? $problem['title'] ?? 'PackForge API request failed with status '.$status);
        $errors = array_values(array_filter(
            (array) ($problem['errors'] ?? []),
            fn ($e) => is_array($e) && isset($e['param'], $e['message']),
        ));

        return match (true) {
            $status === 401 => new AuthenticationException($message, $status, $code),
            $status === 429 => new RateLimitedException($message, $status, $code, [], null, (int) ($response->header('Retry-After') ?: 1)),
            $status === 404 => new NotFoundException($message, $status, $code),
            $code === 'invalid_params' => new ValidationException($message, $status, $code, $errors),
            default => new self($message, $status, $code, $errors),
        };
    }
}
