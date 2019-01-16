<?php

namespace Marcz\Phar\NautPie;

use Symfony\Component\Console\Command\Command;
use Symfony\Component\Console\Input\InputArgument;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Output\OutputInterface;

class DeployNautCommand extends Command
{
    use CurlFetch;

    protected static $defaultName = 'deploy:naut';
    private $description = 'Sends API requests to DeployNaut';

    protected $myOptions = [
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
    ];

    protected function configure()
    {
        $this->addArgument('action', InputArgument::OPTIONAL, 'Command action');

        $this->setOptions($this->myOptions);
    }

    protected function execute(InputInterface $input, OutputInterface $output)
    {
        $action = $input->getArgument('action');
        $endPoint = $this->endPoint ?: getenv('NAUT_ENDPOINT');

        if (!$action || !$endPoint) {
            // white text on a red background
            $output->writeln('<error> Missing Action or End Point </error>');
            // Greater than zero is an error
            return 1;
        }

        $this->setEndpoint($endPoint);
        list($dashUser, $dashToken) = $this->checkEnvs('DASH_USER', 'DASH_TOKEN');
        $this->setUsernameAndPassword($dashUser, $dashToken);

        try {
            $response = $this->executeAction($action);
        } catch (\Exception $e) {
            $this->output->writeln('<error> [Error] ' . $e->getMessage() . ' </error>');
            // Greater than zero is an error
            return 1;
        }

        $output->writeln('Response:'. var_export($response, 1));
    }

    public function doCreateDeployment()
    {
        $stack = $this->getOption('stack');
        $environment = $this->getOption('environment');
        $ref = $this->getOption('ref');
        $refType = $this->getOption('ref_type');

        if ($stack && $environment && $refType) {
            $details = new DeploymentDetails([
                'ref_type' => $refType,
                'ref' => $this->getOption('ref') ?: null,
                'title' => $this->getOption('title') ?: null,
                'summary' => $this->getOption('summary') ?: null,
            ]);

            $startDate = $this->getOption('startDate');
            if ($startDate) {
                $details->scheduleToStart($startDate);
            }

            $bypassAndStart = $this->getOption('bypass_and_start');
            if ($bypassAndStart && !$this->isProductionEnvironment($environment)) {
                $details->bypassAndStart($bypassAndStart);
            }

            $redeploy = $this->getOption('redeploy');
            if ($redeploy && !$this->isProductionEnvironment($environment)) {
                $details->redeploy($redeploy);
            }

            if (!$ref && !$redeploy && $refType !== 'promote_from_uat') {
                throw new \Exception('[Action:CreateDeployment] Requires ref option', 1);
            }

            $relativeUrl = sprintf(
                'project/%s/environment/%s/deploys',
                $stack,
                $environment
            );

            return $this->fetchUrl($relativeUrl, 'POST', $details->values());
        } else {
            throw new \Exception('[Action:CreateDeployment] Requires stack, environment and reference type', 1);
        }
    }

    public  function doGitFetch()
    {
        $stack = $this->getOption('stack');
        if ($stack) {
            $relativeUrl = sprintf('project/%s/git/fetches', $stack);
            $response = $this->fetchUrl($relativeUrl, 'POST');
        } else {
            throw new \Exception('[Action:GitFetch] Requires Stack', 1);
        }

        if ($response['status'] !== 202) {
            throw new \Exception('[Error:Git Fetch] ' . var_export($response,1), 1);
        }

        $relativeUrl = sprintf(
            'project/%s/git/fetches/%s',
            $stack,
            $response['body']['data']['id']
        );

        $isWaiting = true;
        do {
            $this->warning('Waiting for 5 seconds...');
            sleep(5);

            $response = $this->fetchUrl($relativeUrl);
            if ($response['status'] === 200
                && $response['body']['data']['attributes']['status'] === 'Complete'
            ) {
                $this->success('Git Fetch Completed');
                $isWaiting = false;
            }
        } while ($isWaiting);
    }

    public function doGetDeployments()
    {
        $startDate = $this->getOption('startDate') ?: '-1 month';
        $stack = $this->getOption('stack');
        $environment = $this->getOption('environment');
        $deployments = [];

        if ($stack && $environment) {
            $response = $this->fetchDeployments($stack, $environment, $startDate);
            $deployments = $response['body']['data'];
        } else {
            throw new \Exception('[Action:Deployments] Requires Stack and Environment', 1);
        }

        $commitSha = $this->getOption('commit');
        if ($commitSha) {
            $commitKey = strlen($commitSha) === 7 ? 'short_sha' : 'sha';
            $acceptedStates = ['New', 'Submitted', 'Approved', 'Queued', 'Deploying', 'Completed'];
            $deployments = array_filter(
                $deployments,
                function ($item) use ($commitSha, $commitKey, $acceptedStates) {
                    return $item['attributes'][$commitKey] === $commitSha
                        && in_array(
                            $item['attributes']['state'],
                            $acceptedStates
                        );
                }
            );
        }

        usort(
            $deployments,
            function($itemA, $itemB) {
                if ((int) $itemA['id'] === (int) $itemB['id']) {
                    return 0;
                }
                // Will sort the collection from most recent first
                return ((int) $itemA['id'] > (int) $itemB['id']) ? -1 : 1;
            }
        );

        return array_values($deployments);
    }

    public function doLastDeployment()
    {
        return reset($this->doDeployments());
    }

    public function fetchDeployments($stack, $environment, $startDate = '-1 year', $filters = [])
    {
        $filters = array_merge($filters, ['datestarted_from_unix' => strtotime($startDate)]);
        $relativeUrl = sprintf(
            'project/%s/environment/%s/deploys?%s',
            $stack,
            $environment,
            http_build_query($filters)
        );

        $response = $this->fetchUrl($relativeUrl);

        if ($response['status'] !== 200) {
            throw new \Exception(var_export($response['body'], 1), 1);
        }

        return $response;
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
