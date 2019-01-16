<?php
namespace Marcz\Phar\NautPie;

use Dotenv\Dotenv;
use GuzzleHttp\Stream\Stream;
use GuzzleHttp\Ring\Client\CurlHandler;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Output\OutputInterface;
use Symfony\Component\Console\Input\InputOption;

trait CurlFetch
{
    protected $handler;
    protected $data = [];
    protected $input;
    protected $output;
    protected $scheme = 'https';
    protected $contentType = 'application/json';
    protected $endPoint;
    protected $authorization;
    protected $options = [];

    public function setHandler($handler)
    {
        $this->handler = $handler;
    }

    public function setContentType($type)
    {
        $this->contentType = $type;

        if (isset($this->data['headers'])) {
            $this->data['headers']['Content-Type'] = [$type];
        }
    }

    public function setAuthorization($authString)
    {
        $this->authorization = $authString;

        if (isset($this->data['headers'])) {
            $this->data['headers']['Authorization'] = [
                sprintf(
                    'Basic %s',
                    base64_encode($this->authorization)
                )
            ];
        }
    }

    public function setUsernameAndPassword($username, $password)
    {
        $this->setAuthorization($username . ':' . $password);
    }

    public function setEndpoint($endPoint)
    {
        $this->endPoint = $endPoint;

        if (isset($this->data['headers'])) {
            $this->data['headers']['host'] = [parse_url($this->endPoint, PHP_URL_HOST)];
        }
    }

    public function setScheme($https)
    {
        $this->scheme = $https;

        if (isset($this->data['scheme'])) {
            $this->data['scheme'] = [$https];
        }
    }

    /**
     * Curl handler request information
     */
    public function curlSetup()
    {
        if (!$this->data) {
            $this->data = [
                'http_method' => 'GET',
                'scheme' => $this->scheme,
                'uri' => parse_url($this->endPoint, PHP_URL_PATH),
                'headers' => [
                    'Accept' => ['application/json'],
                    'Content-Type' => [$this->contentType],
                    'host' => [parse_url($this->endPoint, PHP_URL_HOST)],
                ],
                'client' => [
                    'timeout' => 120.0,
                    'curl' => [
                        CURLOPT_SSL_VERIFYHOST => 0,
                        CURLOPT_SSL_VERIFYPEER => false,
                        CURLOPT_FOLLOWLOCATION => true,
                    ],
                ],
            ];
        }

        if (!$this->handler) {
            $this->handler = new CurlHandler();
        }

        if ($this->authorization) {
            $this->setAuthorization($this->authorization);
        }

        return $this->curlData();
    }

    public function curlData()
    {
        return $this->data;
    }

    public function resetCurlData()
    {
        $this->data = [];
        $this->curlSetup();

        return $this->curlData();
    }

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

    /**
     * Initializes the command after the input has been bound and before the input
     * is validated.
     *
     * This is mainly useful when a lot of commands extends one main command
     * where some things need to be initialized based on the input arguments and options.
     *
     * @see InputInterface::bind()
     * @see InputInterface::validate()
     */
    protected function initialize(InputInterface $input, OutputInterface $output)
    {
        $this->input = $input;
        $this->output = $output;

        $this->loadEnvironment();
        $this->curlSetup();
    }

    public function fetchUrl($relativeUrl, $httpMethod = 'GET', $body = null)
    {
        $endPoint = $this->endPoint;
        $uri = sprintf(
            '%s/%s',
            parse_url($endPoint, PHP_URL_PATH),
            $relativeUrl
        );

        $data = $this->data;
        $data['uri'] = $uri;
        $data['http_method'] = $httpMethod;

        if ($httpMethod === 'HEAD') {
            $data['client']['curl'][CURLOPT_NOBODY] = true;
            $data['client']['curl'][CURLOPT_HEADER] = true;
        }

        if ($body) {
            if ($this->contentType === 'application/x-www-form-urlencoded') {
                $data['body'] = http_build_query($body);
            } else {
                $data['body'] = json_encode($body, JSON_PRESERVE_ZERO_FRACTION);
            }
        }

        $this->warning('Sending request with: ' . $uri);
        // $this->message($data);

        $client = $this->handler;
        $response = $client($data);
        $stream = Stream::factory($response['body']);
        $response['body'] = json_decode($stream->getContents(), true);

        return $response;
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
            $envs[] = $value;

            if (empty($value)) {
                throw new \Exception('[Required:ENV] ' .$env. ' is missing.', 1);
            }
        }

        return $envs;
    }

    public function getOption($name)
    {
        return $this->input->getOption($name);
    }

    public function getOptions()
    {
        return array_intersect_key($this->input->getOptions(), $this->options);
    }

    public function setOptions($options, $defaultValue = null)
    {
        $this->options = $options;

        foreach ($this->options as $option => $description) {

            $this->addOption($option, null, InputOption::VALUE_OPTIONAL, $description, $defaultValue);
        }

        return $this->options;
    }

    public function executeAction($action, $parameters = [])
    {
        return call_user_func_array([$this, 'do' . ucfirst($action)], $parameters);
    }

    public function doFetch()
    {
        $relativeUrl = $this->getOption('url');

        if (!$relativeUrl) {
            throw new \Exception('[Action:Fetch] Requires relative url option', 1);
        }

        return $this->fetchUrl($relativeUrl);
    }

    public function warning($message)
    {
        if ($this->output) {
            $this->output->writeln('<fg=red;bg=yellow;> '. $message .' </>');
        } else {
            $this->message($message);
        }
    }

    public function success($message)
    {
        if ($this->output) {
            $this->output->writeln('<fg=black;bg=green;> '. $message .' </>');
        } else {
            $this->message($message);
        }
    }

    public function message($message) {
        $message = is_array($message) ? var_export($message, 1) : $message;
        if ($this->output) {
            $this->output->writeln('<info> '. $message .' </>');
        } else {
            fwrite(STDERR, print_r([$message], true));
        }
    }
}
