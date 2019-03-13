<?php

namespace Marcz\Phar\NautPie\Tests;

use GuzzleHttp\Ring\Client\MockHandler;
use Marcz\Phar\NautPie\DeployNautCommand;
use PHPUnit\Framework\TestCase;
use Symfony\Component\Console\Application;
use Symfony\Component\Console\Tester\CommandTester;

/**
 * @internal
 * @coversNothing
 */
final class DeployNautCommandTest extends TestCase
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

    public function testConfigure()
    {
        $command = $this->app->find('deploy:naut');
        $definition = $command->getDefinition();
        $arguments = $definition->getArguments();
        $action = reset($arguments);

        $this->assertCount(1, $arguments);
        $this->assertTrue($action->isRequired());
        $this->assertSame('action', $action->getName());
        $this->assertSame('Command action', $action->getDescription());

        $myOptions = [
            'url' => '[Optional] URL',
            'commit' => '[Optional] Git commit SHA',
            'stack' => '[Optional] Project stack',
            'environment' => '[Optional] Stack environment',
            'startDate' => '[Optional] Start date',
            'title' => '[Optional] Deployment title',
            'summary' => '[Optional] Deployment summary',
            'redeploy' => '[Optional] Redeploy last deployment',
            'ref' => '[Optional] Deployment Reference',
            'ref_type' => '[Optional] Deployment Type',
            'bypass_and_start' => '[Optional] Deployment bypass and start',
            'deploy_id' => '[Optional] Deployment ID',
            'should_wait' => '[Optional] Wait for deployment to finish',
        ];
        $options = $definition->getOptions();

        $this->assertSame(array_keys($options), array_keys($myOptions));

        foreach ($options as $option) {
            $key = $option->getName();
            $this->assertSame($option->getDescription(), $myOptions[$key]);
            $this->assertTrue($option->isValueOptional());
        }
    }

    public function testSampleSuccess()
    {
        $application = $this->app;
        $command = $application->find('deploy:naut');
        $commandTester = new CommandTester($command);
        $commandTester->execute([
            'command' => $command->getName(),
            'action' => 'SampleSuccess',
        ]);

        // the output of the command in the console
        $output = $commandTester->getDisplay();

        $this->assertContains('[Action:Success] Response successful.', $output);
    }

    public function testSampleFail()
    {
        $application = $this->app;
        $command = $application->find('deploy:naut');
        $commandTester = new CommandTester($command);
        $commandTester->execute([
            'command' => $command->getName(),
            'action' => 'SampleFail',
        ]);

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
                'whoami' => 'marcz@example.com',
                'now' => '2017-05-09 11:57:00',
            ],
        ];

        $handler = new MockHandler(
            [
                'status' => 200,
                'reason' => 'OK',
                'body' => json_encode($expectedReturnedData),
            ]
        );
        $command->setHandler($handler);
        $commandTester = new CommandTester($command);
        $commandTester->execute(
            [
                'command' => $command->getName(),
                'action' => 'Fetch',
                '--url' => 'meta',
            ]
        );

        // the output of the command in the console
        $output = $commandTester->getDisplay();
        fwrite(STDERR, print_r(['testCurlFetch', var_export($output, 1)], true));
        // $this->assertContains('Sending request with: /naut/meta', $output);

        // $response = $command->fetchUrl('meta');

        // $this->assertSame(200, $response['status']);
        // $this->assertSame([], $response['headers']);
        // $this->assertSame($expectedReturnedData, $response['body']);
    }

    public function testBadCurlFetch()
    {
        $this->expectException(\Exception::class);
        $this->expectExceptionCode(400);

        $command = $this->command;
        $data = $command->resetCurlData();
        $expectedReturnedData = [
            'errors' => [[
                'status' => '400',
                'title' => 'ref_type "" given but this is not supported',
            ]],
        ];

        $handler = new MockHandler(
            [
                'status' => 400,
                'reason' => 'Bad Request',
                'body' => json_encode($expectedReturnedData),
            ]
        );
        $command->setHandler($handler);
        $commandTester = new CommandTester($command);
        $commandTester->execute(
            [
                'command' => $command->getName(),
                'action' => 'Fetch',
                '--url' => 'meta',
            ]
        );

        // the output of the command in the console
        $output = $commandTester->getDisplay();
        fwrite(STDERR, print_r(['testBadCurlFetch', var_export($output, 1)], true));
        // $this->assertContains('Sending request with: /naut/meta', $output);
        // $this->assertContains(json_encode($expectedReturnedData), $output);

        // $response = $command->fetchUrl('meta');
    }
}
