<?php
namespace Marcz\Phar\NautPie;

use Dotenv\Dotenv;
use GuzzleHttp\Stream\Stream;
use GuzzleHttp\Ring\Client\CurlHandler;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Output\OutputInterface;
use Symfony\Component\Console\Input\InputOption;

trait CommandsHelper
{
    /**
     * Executes a named method using a convention of 'do' + function name
     * @param  string $action     Action Name which translates to method name
     * @param  array  $parameters Passed arguments to the method
     * @return string             Response or JSON-encoded string
     */
    public function executeAction($action, $parameters = [])
    {
        return call_user_func_array([$this, 'do' . ucfirst($action)], $parameters);
    }

    public function doSampleSuccess()
    {
        $message = '[Action:Success] Response successful.';
        $this->success($message);

        return $message;
    }

    public  function doSampleFail()
    {
        throw new \Exception('[Action:Fail] Has failed.', 1);
    }
}
