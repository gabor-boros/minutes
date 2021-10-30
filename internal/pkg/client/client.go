package client

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"io/ioutil"
	"net/http"
	netURL "net/url"
	"os/exec"
	"time"
)

const (
	// DefaultRequestTimeout sets the timeout for the HTTP requests or command
	// executions.
	DefaultRequestTimeout = time.Second * 30
)

var (
	// ErrNoBaseURL returns when HTTP based clients has no BaseURL set, but its
	// `URL()` method was called.
	ErrNoBaseURL = errors.New("no BaseURL provided")
	// ErrInvalidBasicAuth returns if any of the provided basic auth parameters
	// are empty.
	ErrInvalidBasicAuth = errors.New("invalid basic auth params provided")
	// ErrInvalidTokenAuth returns if the provided token is empty.
	ErrInvalidTokenAuth = errors.New("invalid token auth params provided")
)

// BaseClientOpts specifies the common options the clients are using.
// When a client needs other options as well, it composes a new set of options
// using BaseClientOpts.
type BaseClientOpts struct {
	// TagsAsTasks defines to use tag names to determine the task.
	// Using TagsAsTasks can be useful if the user's workflow involves
	// splitting activity across multiple tasks, or when the user has no option
	// to set multiple tasks for a single activity.
	//
	// This option must be used in conjunction with TagsAsTasksRegex option.
	TagsAsTasks bool
	// TagsAsTasksRegex sets the regular expression used for extracting tasks
	// from the list of tags.
	//
	// This option must be used in conjunction with TagsAsTasks option.
	TagsAsTasksRegex string
	// Timeout sets the timeout for the client to execute a request.
	// In the case of HTTP clients, the timeout is applied on the HTTP request,
	// while in the case of CLI based clients it will be applied on the command
	// execution.
	Timeout time.Duration
}

// Authenticator is responsible for setting the necessary parameters for
// authentication on the request.
type Authenticator interface {
	// SetAuthHeader sets the auth header on HTTP requests before the HTTPClient
	// sends it.
	SetAuthHeader(req *http.Request)
}

// BasicAuth represents the required parameters for username and password based
// authentication
type BasicAuth struct {
	Username string
	Password string
}

func (a *BasicAuth) SetAuthHeader(req *http.Request) {
	req.SetBasicAuth(a.Username, a.Password)
}

// NewBasicAuth returns a new BasicAuth that implements Authenticator.
func NewBasicAuth(username string, password string) (Authenticator, error) {
	if username == "" || password == "" {
		return nil, ErrInvalidBasicAuth
	}

	return &BasicAuth{
		Username: username,
		Password: password,
	}, nil
}

// TokenAuth represents the required parameters for token based authentication.
type TokenAuth struct {
	Header    string
	TokenName string
	Token     string
}

func (a *TokenAuth) SetAuthHeader(req *http.Request) {
	token := a.Token

	if a.TokenName != "" {
		token = a.TokenName + " " + token
	}

	req.Header.Set(a.Header, token)
}

// NewTokenAuth returns a new TokenAuth that implements Authenticator. If the
// header name is not set, the standard "Authorization" header will be used.
func NewTokenAuth(header string, tokenName string, token string) (Authenticator, error) {
	if token == "" {
		return nil, ErrInvalidTokenAuth
	}

	if header == "" {
		header = "Authorization"
	}

	return &TokenAuth{
		Header:    header,
		TokenName: tokenName,
		Token:     token,
	}, nil
}

// CLIExecuteOpts represents the options that CLI client's Execute method
// receives.
type CLIExecuteOpts struct {
	Timeout time.Duration
}

// CLIClient implements a client that communicates with a CLI tool.
// The CommandArguments parameter is not used by CLIClient, but those structs
// that uses it for composition.
type CLIClient struct {
	Command            string
	CommandArguments   []string
	CommandCtxExecutor func(ctx context.Context, name string, arg ...string) *exec.Cmd
}

// Execute runs the given CLI command with the specified arguments.
func (c *CLIClient) Execute(ctx context.Context, arguments []string, opts *CLIExecuteOpts) ([]byte, error) {
	ctxWithTimeout, cancel := ctxWithTimeout(ctx, opts.Timeout)
	defer cancel()

	return c.CommandCtxExecutor(ctxWithTimeout, c.Command, arguments...).Output() // #nosec G204
}

// HTTPRequestOpts represents the call options for an HTTP request, fired by the
// HTTPClient when `Call` method is called.
type HTTPRequestOpts struct {
	Method  string
	Url     string
	Data    interface{}
	Headers map[string]string
	Auth    Authenticator
	Timeout time.Duration
}

// HTTPClient implements a client that communicates with the server over HTTP.
type HTTPClient struct {
	Client  *http.Client
	BaseURL *netURL.URL
}

// URL returns the BaseURL combined with the provided params as query params if
// the BaseURL is set. Otherwise, it returns an `ErrNoBaseURL` error.
func (c *HTTPClient) URL(path string, params map[string]string) (string, error) {
	if c.BaseURL == nil {
		return "", ErrNoBaseURL
	}

	urlPath, err := netURL.Parse(path)
	if err != nil {
		return "", err
	}

	url := c.BaseURL.ResolveReference(urlPath)

	query := url.Query()

	for key, val := range params {
		query.Add(key, val)
	}

	url.RawQuery = query.Encode()
	return url.String(), nil
}

// Call fires an HTTP request with the given method and body (in its body) to
// the API URL returned by the `URL` method.
func (c *HTTPClient) Call(ctx context.Context, opts *HTTPRequestOpts) ([]byte, error) {
	ctxWithTimeout, cancel := ctxWithTimeout(ctx, opts.Timeout)
	defer cancel()

	req, err := c.newRequest(ctxWithTimeout, opts)
	if err != nil {
		return nil, err
	}

	resp, err := c.sendRequest(c.Client, req)
	if err != nil {
		return nil, err
	}

	return ioutil.ReadAll(resp.Body)
}

func (c *HTTPClient) newRequest(ctx context.Context, opts *HTTPRequestOpts) (*http.Request, error) {
	var err error
	var body []byte

	if opts.Data != nil {
		body, err = json.Marshal(opts.Data)
		if err != nil {
			return nil, err
		}
	}

	req, err := http.NewRequestWithContext(ctx, opts.Method, opts.Url, bytes.NewBuffer(body))
	if err != nil {
		return nil, err
	}

	for key, val := range opts.Headers {
		req.Header.Set(key, val)
	}

	if opts.Auth != nil {
		opts.Auth.SetAuthHeader(req)
	}

	return req, err
}

func (c *HTTPClient) sendRequest(httpClient *http.Client, req *http.Request) (*http.Response, error) {
	// Set a default HTTP client if no clients were set
	if httpClient == nil {
		httpClient = http.DefaultClient
	}

	resp, err := httpClient.Do(req)
	if err != nil {
		return nil, err
	}

	// If the response wasn't successful, return an error containing the error code
	// https://developer.mozilla.org/en-US/docs/Web/HTTP/Status#successful_responses
	if !(resp.StatusCode >= 200 && resp.StatusCode < 300) {
		errBody, err := io.ReadAll(resp.Body)
		if err != nil {
			return nil, err
		}

		return nil, fmt.Errorf("%d: %s", resp.StatusCode, string(errBody))
	}

	return resp, nil
}

func ctxWithTimeout(ctx context.Context, timeout time.Duration) (context.Context, func()) {
	ctxTimeout := DefaultRequestTimeout
	if timeout > 0 {
		ctxTimeout = timeout
	}

	return context.WithTimeout(ctx, ctxTimeout)
}
