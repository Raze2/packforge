<?php

namespace PackForge\Laravel\Exceptions;

use Throwable;

/** Too many requests for this API key (HTTP 429). */
class RateLimitedException extends PackForgeException
{
    public function __construct(
        string $message,
        int $status = 429,
        string $problemCode = 'rate_limited',
        array $errors = [],
        ?Throwable $previous = null,
        public readonly int $retryAfter = 1,
    ) {
        parent::__construct($message, $status, $problemCode, $errors, $previous);
    }
}
