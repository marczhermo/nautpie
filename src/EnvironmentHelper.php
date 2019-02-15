<?php
namespace Marcz\Phar\NautPie;

use Dotenv\Dotenv;
use GuzzleHttp\Stream\Stream;
use GuzzleHttp\Ring\Client\CurlHandler;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Output\OutputInterface;
use Symfony\Component\Console\Input\InputOption;

trait EnvironmentHelper
{
    /**
     *  Loads .env file safely
     */
    public function loadEnvironment()
    {
        $pharDir = dirname(__DIR__);
        $translate = ['phar://' => '', '/nautpie.phar' => ''];
        $baseSnippetsPath = strtr($pharDir, $translate);
        chdir($baseSnippetsPath);

        $dotenv = new Dotenv($baseSnippetsPath);
        $dotenv->safeLoad();

        $this->message('Path: ' . $baseSnippetsPath);
    }

    public function isProductionEnvironment($environment)
    {
        return in_array(strtolower($environment), ['prod', 'production'], true);
    }

    public function checkEnvs()
    {
        $envs = [];
        foreach (func_get_args() as $env) {
            $value = getenv($env);

            if (empty($value)) {
                throw new \Exception('[Required:ENV] ' .$env. ' is missing.', 1);
            }

            $envs[] = $value;
        }

        return $envs;
    }
}
