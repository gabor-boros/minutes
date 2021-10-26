package client_test

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/http/httptest"
	"net/url"
	"os"
	"os/exec"
	"reflect"
	"strconv"
	"testing"

	"github.com/gabor-boros/minutes/internal/pkg/client"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

var (
	mockedExitCode int
	mockedStdout   string
)

type testData struct {
	Message string `json:"message"`
}

func getDataType(data interface{}) (res string) {
	t := reflect.TypeOf(data)

	for t.Kind() == reflect.Ptr {
		t = t.Elem()
		res += "*"
	}

	return res + t.Name()
}

func mockedExecCommand(_ context.Context, command string, args ...string) *exec.Cmd {
	arguments := []string{"-test.run=TestExecCommandHelper", "--", command}
	arguments = append(arguments, args...)
	cmd := exec.Command(os.Args[0], arguments...)

	cmd.Env = []string{"GO_TEST_HELPER_PROCESS=1",
		"STDOUT=" + mockedStdout,
		"EXIT_CODE=" + strconv.Itoa(mockedExitCode),
	}

	return cmd
}

type mockServerOpts struct {
	Path        string
	Method      string
	StatusCode  int
	Headers     map[string]string
	RequestData interface{}
}

func mockServer(t *testing.T, e *mockServerOpts) *httptest.Server {
	return httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		require.Equal(t, e.Method, r.Method, "API call methods are not matching")
		require.Equal(t, e.Path, r.URL.Path, "API call URLs are not matching")

		for key, values := range r.Header {
			expected := values[0]
			actual, ok := e.Headers[key]

			assert.True(t, ok, fmt.Sprintf("header key \"%s\" is not set", key))
			require.Equal(t, expected, actual)
		}

		if e.RequestData != nil {
			var data interface{}

			switch dataType := getDataType(e.RequestData); dataType {
			case "*testData":
				data = e.RequestData.(*testData)
			default:
				t.Fatalf("%s is not a known data type", dataType)
			}

			// Parse the request
			if err := json.NewDecoder(r.Body).Decode(&data); err != nil {
				t.Fatal(err)
			}
		}

		w.WriteHeader(e.StatusCode)
	}))
}

func newMockServer(t *testing.T, opts *mockServerOpts) *httptest.Server {
	if opts.Headers == nil {
		opts.Headers = map[string]string{}
	}

	defaultHeaders := map[string]string{
		"User-Agent":      "Go-http-client/1.1",
		"Accept-Encoding": "gzip",
	}

	// Set default headers if not set
	for key, val := range defaultHeaders {
		if _, ok := opts.Headers[key]; !ok {
			opts.Headers[key] = val
		}
	}

	mockServer := mockServer(t, opts)
	require.NotNil(t, mockServer, "cannot create mock server")
	return mockServer
}

func TestBasicAuth(t *testing.T) {
	req, err := http.NewRequest(http.MethodGet, "", nil)
	require.Nil(t, err)

	_, _, ok := req.BasicAuth()
	require.False(t, ok)

	auth, err := client.NewBasicAuth("steve", "rogers")
	require.Nil(t, err)

	auth.SetAuthHeader(req)

	username, password, ok := req.BasicAuth()
	assert.True(t, ok)
	require.Equal(t, username, "steve")
	require.Equal(t, password, "rogers")
}

func TestBasicAuth_Invalid(t *testing.T) {
	var err error

	_, err = client.NewBasicAuth("", "")
	require.ErrorIs(t, err, client.ErrInvalidBasicAuth)

	_, err = client.NewBasicAuth("steve", "")
	require.ErrorIs(t, err, client.ErrInvalidBasicAuth)

	_, err = client.NewBasicAuth("", "rogers")
	require.ErrorIs(t, err, client.ErrInvalidBasicAuth)
}

func TestTokenAuth(t *testing.T) {
	header := "X-API-Token"

	req, err := http.NewRequest(http.MethodGet, "", nil)
	require.Nil(t, err)
	require.Equal(t, req.Header.Get(header), "")

	auth, err := client.NewTokenAuth(header, "", "the-strongest-avenger")
	require.Nil(t, err)

	auth.SetAuthHeader(req)
	require.Equal(t, req.Header.Get(header), "the-strongest-avenger")
}

func TestTokenAuth_FallbackHeader(t *testing.T) {
	header := "Authorization"

	req, err := http.NewRequest(http.MethodGet, "", nil)
	require.Nil(t, err)
	require.Equal(t, req.Header.Get(header), "")

	auth, err := client.NewTokenAuth("", "", "the-strongest-avenger")
	require.Nil(t, err)

	auth.SetAuthHeader(req)
	require.Equal(t, req.Header.Get(header), "the-strongest-avenger")
}

func TestTokenAuth_Invalid(t *testing.T) {
	var err error

	_, err = client.NewTokenAuth("", "", "")
	require.ErrorIs(t, err, client.ErrInvalidTokenAuth)
}

// TestExecCommandHelper is a helper test case that will be called by `mockedExecCommand`.
// This workaround is needed to be able to "mock" system calls.
func TestExecCommandHelper(t *testing.T) {
	// Not executed by the mocked command function, so return
	if os.Getenv("GO_TEST_HELPER_PROCESS") != "1" {
		return
	}

	_, _ = fmt.Fprint(os.Stdout, os.Getenv("STDOUT"))
	exitCode, _ := strconv.Atoi(os.Getenv("EXIT_CODE"))
	os.Exit(exitCode)
}

func TestCLIClient_Execute(t *testing.T) {
	mockedExitCode = 0
	mockedStdout = "[]"

	cliClient := client.CLIClient{
		Command:            "some-command",
		CommandArguments:   []string{},
		CommandCtxExecutor: mockedExecCommand,
	}

	out, err := cliClient.Execute(context.Background(), cliClient.CommandArguments, &client.CLIExecuteOpts{})
	require.Nil(t, err)
	require.Equal(t, string(out), "[]")
}

func TestHTTPClient_URL(t *testing.T) {
	baseURL, err := url.Parse("https://example.com")
	require.Nil(t, err)

	httpClient := client.HTTPClient{
		Client:  http.DefaultClient,
		BaseURL: baseURL,
	}

	urlParams := map[string]string{
		"param1": "value1",
		"param2": "value2",
	}

	expectedURL := "https://example.com/test?param1=value1&param2=value2"
	combinedURL, err := httpClient.URL("/test", urlParams)

	require.Nil(t, err)
	require.Equal(t, combinedURL, expectedURL)
}

func TestHTTPClient_URL_Invalid(t *testing.T) {
	baseURL, err := url.Parse("https://example.com")
	require.Nil(t, err)

	httpClient := client.HTTPClient{
		Client:  http.DefaultClient,
		BaseURL: baseURL,
	}

	_, err = httpClient.URL("https://not a real path", map[string]string{})
	require.Error(t, err)
}

func TestHTTPClient_URL_NoPath(t *testing.T) {
	baseURL, err := url.Parse("https://example.com")
	require.Nil(t, err)

	httpClient := client.HTTPClient{
		Client:  http.DefaultClient,
		BaseURL: baseURL,
	}

	urlParams := map[string]string{
		"param1": "value1",
		"param2": "value2",
	}

	expectedURL := "https://example.com?param1=value1&param2=value2"
	combinedURL, err := httpClient.URL("", urlParams)

	require.Nil(t, err)
	require.Equal(t, combinedURL, expectedURL)
}

func TestHTTPClient_URL_Empty(t *testing.T) {
	httpClient := client.HTTPClient{
		Client: http.DefaultClient,
	}

	combinedURL, err := httpClient.URL("", map[string]string{})

	require.ErrorIs(t, err, client.ErrNoBaseURL)
	require.Empty(t, combinedURL)
}

func TestHTTPClient_URL_NoBaseURL(t *testing.T) {
	httpClient := client.HTTPClient{
		Client: http.DefaultClient,
	}

	urlParams := map[string]string{
		"param1": "value1",
		"param2": "value2",
	}

	combinedURL, err := httpClient.URL("", urlParams)

	require.ErrorIs(t, err, client.ErrNoBaseURL)
	require.Empty(t, combinedURL)
}

func TestHTTPClient_URL_NoParams(t *testing.T) {
	baseURL, err := url.Parse("https://example.com")
	require.Nil(t, err)

	httpClient := client.HTTPClient{
		Client:  http.DefaultClient,
		BaseURL: baseURL,
	}

	combinedURL, err := httpClient.URL("", map[string]string{})

	require.Nil(t, err)
	require.Equal(t, combinedURL, baseURL.String())
}

func TestHTTPClient_Call(t *testing.T) {
	path := "/endpoint"
	method := http.MethodGet
	headers := map[string]string{
		"Test": "Header",
	}

	mockServer := newMockServer(t, &mockServerOpts{
		Path:       path,
		Method:     method,
		StatusCode: http.StatusOK,
		Headers:    headers,
	})
	defer mockServer.Close()

	baseURL, err := url.Parse(mockServer.URL)
	require.Nil(t, err)

	httpClient := client.HTTPClient{
		Client:  http.DefaultClient,
		BaseURL: baseURL,
	}

	requestURL, err := httpClient.URL(path, map[string]string{})
	require.Nil(t, err)

	resp, err := httpClient.Call(context.Background(), &client.HTTPRequestOpts{
		Method:  method,
		Url:     requestURL,
		Auth:    nil,
		Headers: headers,
		Data:    nil,
	})

	require.Nil(t, err, err)
	require.Equal(t, []byte{}, resp)
}

func TestHTTPClient_Call_NoClientSet(t *testing.T) {
	path := "/endpoint"
	method := http.MethodGet
	headers := map[string]string{
		"Test": "Header",
	}

	mockServer := newMockServer(t, &mockServerOpts{
		Path:       path,
		Method:     method,
		StatusCode: http.StatusOK,
		Headers:    headers,
	})
	defer mockServer.Close()

	baseURL, err := url.Parse(mockServer.URL)
	require.Nil(t, err)

	httpClient := client.HTTPClient{
		BaseURL: baseURL,
	}

	requestURL, err := httpClient.URL(path, map[string]string{})
	require.Nil(t, err)

	resp, err := httpClient.Call(context.Background(), &client.HTTPRequestOpts{
		Method:  method,
		Url:     requestURL,
		Auth:    nil,
		Headers: headers,
		Data:    nil,
	})

	require.Nil(t, err)
	require.Equal(t, []byte{}, resp)
}

func TestHTTPClient_Call_POST(t *testing.T) {
	path := "/endpoint"
	method := http.MethodPost

	mockServer := newMockServer(t, &mockServerOpts{
		Path:       path,
		Method:     method,
		StatusCode: http.StatusOK,
		Headers: map[string]string{
			"Content-Length": "18",
		},
	})
	defer mockServer.Close()

	baseURL, err := url.Parse(mockServer.URL)
	require.Nil(t, err)

	httpClient := client.HTTPClient{
		Client:  http.DefaultClient,
		BaseURL: baseURL,
	}

	requestURL, err := httpClient.URL(path, map[string]string{})
	require.Nil(t, err)

	resp, err := httpClient.Call(context.Background(), &client.HTTPRequestOpts{
		Method:  method,
		Url:     requestURL,
		Auth:    nil,
		Headers: map[string]string{},
		Data: testData{
			Message: "Test",
		},
	})

	require.Nil(t, err)
	require.Equal(t, []byte{}, resp)
}

func TestHTTPClient_Call_Auth(t *testing.T) {
	path := "/endpoint"
	method := http.MethodGet
	headers := map[string]string{
		"Authorization": "Basic c3RldmU6cm9nZXJz",
	}

	mockServer := newMockServer(t, &mockServerOpts{
		Path:       path,
		Method:     method,
		StatusCode: http.StatusOK,
		Headers:    headers,
	})
	defer mockServer.Close()

	baseURL, err := url.Parse(mockServer.URL)
	require.Nil(t, err)

	httpClient := client.HTTPClient{
		Client:  http.DefaultClient,
		BaseURL: baseURL,
	}

	requestURL, err := httpClient.URL(path, map[string]string{})
	require.Nil(t, err)

	authMethod, err := client.NewBasicAuth("steve", "rogers")
	require.Nil(t, err)

	resp, err := httpClient.Call(context.Background(), &client.HTTPRequestOpts{
		Method:  method,
		Url:     requestURL,
		Auth:    authMethod,
		Headers: headers,
		Data:    nil,
	})

	require.Nil(t, err)
	require.Equal(t, []byte{}, resp)
}

func TestHTTPClient_Call_Failure(t *testing.T) {
	path := "/endpoint"
	method := http.MethodGet

	mockServer := newMockServer(t, &mockServerOpts{
		Path:       path,
		Method:     method,
		StatusCode: http.StatusInternalServerError,
	})
	defer mockServer.Close()

	baseURL, err := url.Parse(mockServer.URL)
	require.Nil(t, err)

	httpClient := client.HTTPClient{
		Client:  http.DefaultClient,
		BaseURL: baseURL,
	}

	requestURL, err := httpClient.URL(path, map[string]string{})
	require.Nil(t, err)

	_, err = httpClient.Call(context.Background(), &client.HTTPRequestOpts{
		Method:  method,
		Url:     requestURL,
		Auth:    nil,
		Headers: map[string]string{},
		Data:    nil,
	})

	require.Error(t, err)
}
