<?php

namespace Marcz\Phar\NautPie;

use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputArgument;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Output\OutputInterface;
use Symfony\Component\Console\Input\ArrayInput;

class BitbucketCommand extends Command
{
    use CurlFetch;

    protected static $defaultName = 'ci:bitbucket';
    private $description = 'Bitbucket Pipelines';

    protected $myOptions = [
        'commit' => '[Optional] Git commit SHA',
        'stack' => '[Optional] Project stack',
        'environment' => '[Optional] Stack environment',
        'bypass_and_start' => '[Optional] Deployment bypass and start',
    ];

    protected function configure()
    {
        $this->addArgument('action', InputArgument::OPTIONAL, 'Command action');

        $this->setOptions($this->myOptions);
    }

    protected function execute(InputInterface $input, OutputInterface $output)
    {
        $action = $input->getArgument('action');
        $authorization = getenv('BB_AUTH_STRING');
        $endPoint = $this->endPoint ?: getenv('BB_ENDPOINT');

        if ($action && $authorization && $endPoint) {
            $this->setEndpoint($endPoint);

            $this->setAuthorization($authorization);

            $this->warning($this->endPoint);

            try {
                $response = $this->executeAction($action);
            } catch (\Exception $e) {
                $this->output->writeln('<error> ' . $e->getMessage() . ' </error>');
                // Greater than zero is an error
                return 1;
            }

            $output->writeln('Response:'. var_export($response, 1));
        } else {
            $output->writeln('<error> Missing Action </error>');
            // Greater than zero is an error
            return 1;
        }
    }

    public function doDeployPackage()
    {
        $commit = $this->getOption('commit');
        $stack = $this->getOption('stack');
        $environment = $this->getOption('environment');
        $bypassAndStart = $this->getOption('bypass_and_start') ?: true;
        if ($commit && strlen($commit) === 40 && $stack && $environment) {
            $repoOwner = getenv('BITBUCKET_REPO_OWNER');
            $repoSlug = getenv('BITBUCKET_REPO_SLUG');
            $endPoint = getenv('BB_ENDPOINT');
            $branchName = getenv('BITBUCKET_BRANCH');

            $downloadLink = sprintf(
                '%s/repositories/%s/%s/downloads/%s.tar.gz?access_token=%s',
                $endPoint,
                $repoOwner,
                $repoSlug,
                $commit,
                $this->doAccessToken()
            );

            $command = $this->getApplication()->find('deploy:naut');
            $command->resetCurlData();
            $otherInput = new ArrayInput([
                'command' => 'deploy:naut',
                'action' => 'createDeployment',
                '--stack' => $stack,
                '--environment' => $environment,
                '--ref_type' => 'package',
                '--ref' => $downloadLink,
                '--title' => '[CI:Package] ' . $commit,
                '--summary' => 'Branch:' . $branchName,
                '--bypass_and_start' => $bypassAndStart,
            ]);

            return $command->run($otherInput, $this->output);
        }

        throw new \Exception('[Action:DeployPackage] Requires stack, environment and 40-char commit', 1);
    }

    public function doDeployGitSha()
    {
        $commit = $this->getOption('commit');
        $stack = $this->getOption('stack');
        $environment = $this->getOption('environment');
        $bypassAndStart = $this->getOption('bypass_and_start') ?: true;
        if ($commit && strlen($commit) === 40 && $stack && $environment) {
            $branchName = getenv('BITBUCKET_BRANCH');
            $command = $this->getApplication()->find('deploy:naut');
            $command->resetCurlData();
            $otherInput = new ArrayInput([
                'command' => 'deploy:naut',
                'action' => 'createDeployment',
                '--stack' => $stack,
                '--environment' => $environment,
                '--ref_type' => 'sha',
                '--ref' => $commit,
                '--title' => '[CI:SHA] ' . $commit,
                '--summary' => 'Branch:' . $branchName,
                '--bypass_and_start' => $bypassAndStart,
            ]);

            return $command->run($otherInput, $this->output);
        }

        throw new \Exception('[Action:DeployGitSha] Requires 40-char commit option', 1);
    }

    public function doCreateAccessToken()
    {
        $response = $this->fetchAccessToken();
        $accessToken = $response['body']['access_token'];
        return $accessToken;
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
        $this->setEndpoint('https://bitbucket.org/site');
        $this->setContentType('application/x-www-form-urlencoded');
        $this->setUsernameAndPassword(getenv('BB_CONSUMER_KEY'), getenv('BB_CONSUMER_SECRET'));
        $response = $this->fetchUrl('oauth2/access_token/', 'POST', ['grant_type' => 'client_credentials']);

        if ($response['status'] !== 200) {
            throw new \Exception(var_export($response['body'], 1), 1);
        }

        return $response;
    }
}
