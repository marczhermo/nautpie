<?php

namespace Marcz\Phar\NautPie;

use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputArgument;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Output\OutputInterface;
use Symfony\Component\Console\Input\ArrayInput;
use Symfony\Component\Console\Output\StreamOutput;
use Marcz\Phar\NautPie\DeployNautCommand;

class BitbucketCommand extends Command
{
    use CurlFetch;
    use CheckHelper;

    protected static $defaultName = 'ci:bitbucket';
    private $description = 'Bitbucket Pipelines';

    protected $myOptions = [
        'commit' => '[Optional] Git commit SHA',
        'stack' => '[Optional] Project stack',
        'environment' => '[Optional] Stack environment',
        'title' => '[Optional] Deployment title',
        'summary' => '[Optional] Deployment summary',
        'bypass_and_start' => '[Optional] Deployment bypass and start',
        'deploy_id' => '[Optional] Deployment ID',
        'should_wait' => '[Optional] Wait for deployment to finish',
    ];

    protected function configure()
    {
        $this->addArgument('action', InputArgument::REQUIRED, 'Command action');

        $this->setOptions($this->myOptions);
    }

    protected function execute(InputInterface $input, OutputInterface $output)
    {
        $action = $input->getArgument('action');
        $endPoint = $this->endPoint ?: getenv('BB_ENDPOINT');

        try {
            if (!$action || !$endPoint) {
                throw new \Exception('[Missing] Action or End Point', 1);
            }

            list($authorization) = $this->checkEnvs('BB_AUTH_STRING');
            $this->setEndpoint($endPoint);
            $this->setAuthorization($authorization);

            $response = $this->executeAction($action);
        } catch (\Exception $e) {
            $body = json_decode($e->getMessage(), 1);

            if (is_null($body)) {
                $this->warning($e->getMessage());
                $body = sprintf('"%s"', $e->getMessage());
            }

            $response = [
                'status' => $e->getCode() ?: 1,
                'reason' => 'Bad Request',
                'body'=> $body,
            ];

            $output->writeln(json_encode($response));

            // Greater than zero is an error
            return 1;
        }

        $output->writeln(json_encode($response));
    }

    public function doDeployPackage()
    {
        list($commit, $stack, $environment) = $this->checkRequiredOptions(
            'commit',
            'stack',
            'environment'
        );

        list($repoOwner, $repoSlug, $endPoint, $branchName) = $this->checkEnvs(
            'BITBUCKET_REPO_OWNER',
            'BITBUCKET_REPO_SLUG',
            'BB_ENDPOINT',
            'BITBUCKET_BRANCH'
        );

        $tokenResponse = $this->doCreateAccessToken();
        $accessToken = $tokenResponse['body']['access_token'];

        $downloadLink = sprintf(
            '%s/repositories/%s/%s/downloads/%s.tar.gz?access_token=%s',
            $endPoint,
            $repoOwner,
            $repoSlug,
            $commit,
            $accessToken
        );

        $command = $this->getApplication()->find('deploy:naut');
        $command->resetCurlData();
        $createDeploymentInput = new ArrayInput([
            'command' => 'deploy:naut',
            'action' => 'createDeployment',
            '--stack' => $stack,
            '--environment' => $environment,
            '--ref_type' => 'package',
            '--ref' => $downloadLink,
            '--title' => '[CD:Package] ' . $commit,
            '--summary' => 'Branch:' . $branchName,
            '--bypass_and_start' => $this->getOption('bypass_and_start'),
            '--should_wait' => $this->getOption('should_wait'),
        ]);

        return $command->run($createDeploymentInput, $this->output);
    }

    public function doCreateTag()
    {
        list($commit) = $this->checkRequiredOptions('commit');
        list($repoOwner, $repoSlug) = $this->checkEnvs(
            'BITBUCKET_REPO_OWNER',
            'BITBUCKET_REPO_SLUG'
        );

        $relativeUrl = sprintf(
            'repositories/%s/%s/refs/tags',
            $repoOwner,
            $repoSlug
        );

        $payload = [
            'name' => 'release-rc-' . $commit,
            'target' => ['hash' => $commit],
        ];

        return $this->fetchUrl($relativeUrl, 'POST', $payload);
    }

    public function doCreateAccessToken()
    {
        $response = $this->fetchAccessToken();
        $accessToken = $response['body']['access_token'];
        $this->success($accessToken);

        return $response;
    }

    /**
     * Creates an access token which normally expires in 2 hours
     *
     * curl -X POST -u "ZHqXmhVaDZGLD6paGz:748KNT4MHg8k3a2qvFdgaqQkfvvzMJBt" \
     *     https://bitbucket.org/site/oauth2/access_token \
     *     -d grant_type=client_credentials
     * Then use the access_token so that Platform can download the file from Bitbucket
     * https://api.bitbucket.org/2.0/repositories/marcz/cicd-test/downloads/cd4fd6988796e34a348540f2f9a74249de61b24d.tar.gz?access_token={access_token}
     */
    public function fetchAccessToken()
    {
        list($consumerKey, $consumerSecret) = $this->checkEnvs('BB_CONSUMER_KEY', 'BB_CONSUMER_SECRET');

        $this->setEndpoint('https://bitbucket.org/site');
        $this->setContentType('application/x-www-form-urlencoded');
        $this->setUsernameAndPassword($consumerKey, $consumerSecret);
        $response = $this->fetchUrl('oauth2/access_token/', 'POST', ['grant_type' => 'client_credentials']);

        if ($response['status'] !== 200) {
            throw new \Exception(var_export($response['body'], 1), $response['status']);
        }

        return $response;
    }
}
