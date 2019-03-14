<?php
namespace Marcz\Phar\NautPie;

use Dotenv\Dotenv;
use GuzzleHttp\Stream\Stream;
use GuzzleHttp\Ring\Client\CurlHandler;
use Symfony\Component\Console\Input\InputInterface;
use Symfony\Component\Console\Output\OutputInterface;
use Symfony\Component\Console\Input\InputOption;

trait CurlHelper
{
    protected $handler;
    protected $data = [];
    protected $scheme = 'https';
    protected $contentType = 'application/json';
    protected $endPoint;
    protected $authorization;

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
                $jsonOption = defined('JSON_PRESERVE_ZERO_FRACTION') ? JSON_PRESERVE_ZERO_FRACTION : 0;
                $data['body'] = json_encode($body, $jsonOption);
            }
        }

        $this->warning('Sending request with: ' . $uri);

        $client = $this->handler;
        $response = $client($data);
        $stream = Stream::factory($response['body']);
        $contents = $stream->getContents();
        $response['body'] = json_decode($contents, true);

        return [
            'status' => $response['status'],
            'reason' => $response['reason'],
            'body'=> $response['body'],
        ];
    }

    public function isErrorResponse($statusCode)
    {
        $statusCode = $statusCode ?: 0;

        return $statusCode < 200 || $statusCode > 399;
    }

    public function doFetch()
    {
        $relativeUrl = $this->getOption('url');

        if (!$relativeUrl) {
            throw new \Exception('[Action:Fetch] Requires relative url option', 1);
        }

        return $this->fetchUrl($relativeUrl);
    }
}
