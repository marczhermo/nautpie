## Bitbucket Pipelines

Create a deployment with Git SHA

```
./nautpie.phar ci:bitbucket deployGitSha --stack=example --environment=uat --commit=40char-sha
```

Create a deployment with packaged .tar.gz file

```
./nautpie.phar ci:bitbucket deployPackage --stack=example --environment=uat --commit=40char-sha
```

Create an access token

```
./nautpie.phar ci:bitbucket CreateAccessToken
```
