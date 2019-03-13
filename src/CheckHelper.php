<?php

namespace Marcz\Phar\NautPie;

trait CheckHelper
{
    /**
     * Work-around to use isNotNull (aka isset)  as a callback function
     * @param  mixed $value
     * @return boolean
     */
    public function isNotNull($value)
    {
        // Note: Because this is a language construct and not a function,
        // it cannot be called using variable functions.
        return isset($value);
    }

    /**
     * Checks acceptable boolean string values
     * @param  string|boolean $value Boolean string
     * @return boolean
     */
    public function checkBoolean($value)
    {
        if (in_array(strtolower($value), ['false', 'no', '0'], 1)) {
            return false;
        }

        if (in_array(strtolower($value), ['true', 'yes', '1'], 1)) {
            return true;
        }

        if (is_null($value)) {
            return false;
        }

        if (!is_bool($value)) {
            throw new \Exception('[Boolean] Value not accepted as boolean', 1);
        }

        return (bool) $value;
    }
}
