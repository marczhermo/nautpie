<?php

namespace Marcz\Phar\NautPie\Tests;

use GuzzleHttp\Ring\Client\MockHandler;
use GuzzleHttp\Stream\Stream;
use Marcz\Phar\NautPie\DeployNautCommand;
use Symfony\Component\Console\Tester\CommandTester;
use Symfony\Component\Console\Application;
use PHPUnit\Framework\TestCase;

class DeployNautCommandTest extends TestCase
{
    protected $app;
    protected $command;

    protected function setUp()
    {
        parent::setUp();
        putenv('NAUT_ENDPOINT=https://platform.silverstripe.com/naut');
        putenv('DASH_USER=DASH_USER');
        putenv('DASH_TOKEN=DASH_TOKEN');

        $this->app = new Application('NautPie', '@package_version@');
        $this->command = new DeployNautCommand();
        $this->app->add($this->command);
    }

    public function testSampleSuccess()
    {
        $application = $this->app;
        $command = $application->find('deploy:naut');
        $commandTester = new CommandTester($command);
        $commandTester->execute(array(
            'command'  => $command->getName(),
            'action' => 'SampleSuccess',
        ));

        // the output of the command in the console
        $output = $commandTester->getDisplay();

        $this->assertContains('[Action:Success] Response successful.', $output);
    }

    public function testSampleFail()
    {
        $application = $this->app;
        $command = $application->find('deploy:naut');
        $commandTester = new CommandTester($command);
        $commandTester->execute(array(
            'command'  => $command->getName(),
            'action' => 'SampleFail',
        ));

        // the output of the command in the console
        $output = $commandTester->getDisplay();

        $this->assertContains('[Action:Fail] Has failed.', $output);
    }

    public function testCurlFetch()
    {
        $command = $this->command;
        $data = $command->resetCurlData();
        $expectedReturnedData = [
            'meta' => [
                'whoami' => 'joe@example.com',
                'now' => '2017-05-09 11:57:00',
            ]
        ];

        $handler = new MockHandler(
            [
                'status' => 200,
                'reason' => 'OK',
                'body' => json_encode($expectedReturnedData)
            ]
        );
        $command->setHandler($handler);
        $commandTester = new CommandTester($command);
        $commandTester->execute(
            [
                'command'  => $command->getName(),
                'action' => 'Fetch',
                '--url' => 'meta',
            ]
        );

        // the output of the command in the console
        $output = $commandTester->getDisplay();
        $this->assertContains('Sending request with: /naut/meta', $output);

        $response = $command->fetchUrl('meta');

        $this->assertEquals(200, $response['status']);
        $this->assertEquals([], $response['headers']);
        $this->assertEquals($expectedReturnedData, $response['body']);
    }
}
