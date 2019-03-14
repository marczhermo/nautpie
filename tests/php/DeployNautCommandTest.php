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
        $response = json_decode($commandTester->getDisplay(), 1);
        $this->assertEquals('"[Action:Success] Response successful."', trim($output));
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
        $response = json_decode($commandTester->getDisplay(), 1);

        $this->assertEquals(1, $response['status']);
        $this->assertEquals('Bad Request', $response['reason']);
        $this->assertEquals('"[Action:Fail] Has failed."', $response['body']);
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
        $expectedResponse = [
            'status' => 200,
            'reason' => 'OK',
            'body' => json_encode($expectedReturnedData),
        ];

        $handler = new MockHandler($expectedResponse);
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
        $response = json_decode($commandTester->getDisplay(), 1);
        $this->assertEquals(200, $response['status']);
        $this->assertEquals('OK', $response['reason']);
        $this->assertEquals($expectedReturnedData, $response['body']);
    }

    public function testBadResponseCurlFetch()
    {
        $command = $this->command;
        $data = $command->resetCurlData();
        $expectedReturnedData = [
            'errors' => [[
                'status' => '400',
                'title' => 'ref_type "" given but this is not supported',
            ]],
        ];
        $expectedResponse = [
            'status' => 400,
            'reason' => 'Bad Request',
            'body' => json_encode($expectedReturnedData),
        ];

        $handler = new MockHandler($expectedResponse);
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
        $response = json_decode($commandTester->getDisplay(), 1);
        $this->assertEquals(400, $response['status']);
        $this->assertEquals('Bad Request', $response['reason']);
        $this->assertEquals($expectedReturnedData, $response['body']);
    }

    public function testExceptionThrownOnAction()
    {
        $this->expectException(\Exception::class);
        $this->expectExceptionCode(1);

        $command = $this->command;
        $data = $command->resetCurlData();

        $expectedReturnedData = '[Missing] Action or End Point';
        $expectedResponse = [
            'status' => 400,
            'reason' => 'Bad Request',
            'body' => sprintf('"%s"', $expectedReturnedData),
        ];

        $handler = new MockHandler($expectedResponse);
        $command->setHandler($handler);
        $commandTester = new CommandTester($command);
        $commandTester->execute(
            [
                'command' => $command->getName(),
                'action' => '',
                '--url' => 'meta',
            ]
        );

        // the output of the command in the console
        $response = json_decode($commandTester->getDisplay(), 1);
        $this->assertEquals(1, $response['status']);
        $this->assertEquals('Bad Request', $response['reason']);
        $this->assertEquals(sprintf('"%s"', $expectedReturnedData), $response['body']);

        // This will throw and \Exception
        $this->command->doSampleFail();
    }

    public function testGetDeployments()
    {
        $command = $this->command;
        $data = $command->resetCurlData();
        $expectedReturnedData = file_get_contents(dirname(__FILE__) . '/../fixtures/getDeployments.json');
        $expectedResponse = [
            'status' => 200,
            'reason' => null,
            'body' => $expectedReturnedData,
        ];

        $handler = new MockHandler($expectedResponse);
        $command->setHandler($handler);
        $commandTester = new CommandTester($command);
        $commandTester->execute(
            [
                'command' => $command->getName(),
                'action' => 'getDeployments',
                '--stack' => 'stack',
                '--environment' => 'uat',
            ]
        );

        // the output of the command in the console
        $response = json_decode($commandTester->getDisplay(), 1);

        $this->assertEquals(200, $response['status']);
        $this->assertEquals(null, $response['reason']);
        $this->assertEquals(json_decode($expectedReturnedData, 1), $response['body']);
    }

    public function testLastDeployment()
    {
        $command = $this->command;
        $data = $command->resetCurlData();
        $expectedReturnedData = file_get_contents(dirname(__FILE__) . '/../fixtures/getDeployments.json');
        $expectedResponse = [
            'status' => 200,
            'reason' => null,
            'body' => $expectedReturnedData,
        ];

        $handler = new MockHandler($expectedResponse);
        $command->setHandler($handler);
        $commandTester = new CommandTester($command);
        $commandTester->execute(
            [
                'command' => $command->getName(),
                'action' => 'lastDeployment',
                '--stack' => 'stack',
                '--environment' => 'uat',
            ]
        );

        // the output of the command in the console
        $response = json_decode($commandTester->getDisplay(), 1);

        $this->assertEquals(200, $response['status']);
        $this->assertEquals(null, $response['reason']);

        $first = json_decode($expectedReturnedData, 1);

        $this->assertEquals(reset($first['data']), $response['body']);
    }

    public function testCreateDeployment()
    {
        $command = $this->command;
        $data = $command->resetCurlData();
        $expectedReturnedData = [
            'data' => [
                'type' => 'deployments',
                'id' => '64040',
                'attributes' => [
                    'id' => 640640,
                    'title' => '[CD:Package] COMMIT_HASH'
                ]
            ],
        ];
        $expectedResponse = [
            'status' => 201,
            'reason' => 'Created',
            'body' => json_encode($expectedReturnedData),
        ];

        $handler = new MockHandler($expectedResponse);
        $command->setHandler($handler);
        $commandTester = new CommandTester($command);
        $commandTester->execute(
            [
                'command' => $command->getName(),
                'action' => 'createDeployment',
                '--stack' => 'stack',
                '--environment' => 'uat',
                '--ref_type' => 'branch',
                '--ref' => 'develop',
            ]
        );

        // the output of the command in the console
        $response = json_decode($commandTester->getDisplay(), 1);

        $this->assertEquals(201, $response['status']);
        $this->assertEquals('Created', $response['reason']);
        $this->assertEquals($expectedReturnedData, $response['body']);
    }
}
