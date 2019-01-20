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
    public function executeAction($action, $parameters = [])
    {
        return call_user_func_array([$this, 'do' . ucfirst($action)], $parameters);
    }

    public function doSampleSuccess()
    {
        $this->success('[Action:Success] Response successful.');
    }

    public  function doSampleFail()
    {
        throw new \Exception('[Action:Fail] Has failed.', 1);
    }
}
