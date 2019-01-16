#!/usr/bin/env php
<?php
if (file_exists(__DIR__.'/../vendor/autoload.php')) {
    require __DIR__.'/../vendor/autoload.php';
} else {
    require __DIR__.'/vendor/autoload.php';
}

use Symfony\Component\Console\Application;

$application = new Application('NautPie', '@package_version@');
$application->add(new \Marcz\Phar\NautPie\DeployNautCommand());
$application->add(new Marcz\Phar\NautPie\BitbucketCommand());
$application->run();
