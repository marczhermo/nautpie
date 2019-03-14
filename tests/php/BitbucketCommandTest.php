<?php

namespace Marcz\Phar\NautPie\Tests;

use GuzzleHttp\Ring\Client\MockHandler;
use Marcz\Phar\NautPie\BitbucketCommand;
use Marcz\Phar\NautPie\DeployNautCommand;
use PHPUnit\Framework\TestCase;
use Symfony\Component\Console\Application;
use Symfony\Component\Console\Tester\CommandTester;

/**
 * @internal
 * @coversNothing
 */
final class BitbucketCommandTest extends TestCase
{
    protected $app;
    protected $command;
    protected $deployCommand;

    protected function setUp()
    {
        parent::setUp();
        putenv('NAUT_ENDPOINT=https://platform.silverstripe.com/naut');
        putenv('DASH_USER=DASH_USER');
        putenv('DASH_TOKEN=DASH_TOKEN');
        putenv('BB_ENDPOINT=https://api.bitbucket.org/2.0');
        putenv('BB_AUTH_STRING=marcz:password');
        putenv('BITBUCKET_REPO_OWNER=ssmarco');
        putenv('BITBUCKET_REPO_SLUG=cd-test');
        putenv('BITBUCKET_BRANCH=release/something');
        putenv('BB_CONSUMER_KEY=BB_CONSUMER_KEY');
        putenv('BB_CONSUMER_SECRET=BB_CONSUMER_SECRET');

        $this->app = new Application('NautPie', '@package_version@');
        $this->command = new BitbucketCommand();
        $this->app->add($this->command);
    }

    public function testConfigure()
    {
        $command = $this->app->find('ci:bitbucket');
        $definition = $command->getDefinition();
        $arguments = $definition->getArguments();
        $action = reset($arguments);

        $this->assertCount(1, $arguments);
        $this->assertTrue($action->isRequired());
        $this->assertSame('action', $action->getName());
        $this->assertSame('Command action', $action->getDescription());

        $myOptions = [
            'commit' => '[Optional] Git commit SHA',
            'stack' => '[Optional] Project stack',
            'environment' => '[Optional] Stack environment',
            'title' => '[Optional] Deployment title',
            'summary' => '[Optional] Deployment summary',
            'tag' => '[Optional] Deployment tag',
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

    public function testCreateTag()
    {
        $command = $this->command;
        $data = $command->resetCurlData();
        $expectedReturnedData = ['name' => 'v1.2.34'];
        $expectedResponse = [
            'status' => 201,
            'reason' => null,
            'body' => json_encode($expectedReturnedData),
        ];

        $handler = new MockHandler($expectedResponse);
        $command->setHandler($handler);
        $commandTester = new CommandTester($command);
        $commandTester->execute(
            [
                'command' => $command->getName(),
                'action' => 'CreateTag',
                '--commit' => 'COMMIT_HASH_REQUIRES_40_CHARS_1234567890',
                '--tag' => 'v1.2.34'
            ]
        );

        // the output of the command in the console
        $response = json_decode($commandTester->getDisplay(), 1);

        $this->assertEquals(201, $response['status']);
        $this->assertEquals(null, $response['reason']);
        $this->assertEquals($expectedReturnedData, $response['body']);
    }

    public function testCreateAccessToken()
    {
        $command = $this->command;
        $data = $command->resetCurlData();
        $expectedReturnedData = [
            'access_token' => 'R2uy6EHAKD7K1q3IG9FHf1B4hq9IprTHiLsT0HnA=',
            'scopes' => 'repository',
            'expires_in' => 7200,
            'refresh_token' => 'Sdyr6UewYGxsmgDH78',
            'token_type' => 'bearer',
        ];
        $expectedResponse = [
            'status' => 200,
            'reason' => null,
            'body' => json_encode($expectedReturnedData),
        ];

        $handler = new MockHandler($expectedResponse);
        $command->setHandler($handler);
        $commandTester = new CommandTester($command);
        $commandTester->execute(
            [
                'command' => $command->getName(),
                'action' => 'CreateAccessToken',
            ]
        );

        // the output of the command in the console
        $response = json_decode($commandTester->getDisplay(), 1);

        $this->assertEquals(200, $response['status']);
        $this->assertEquals(null, $response['reason']);
        $this->assertEquals($expectedReturnedData, $response['body']);
    }
}
