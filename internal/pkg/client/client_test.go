package client_test

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"reflect"
	"testing"

	"github.com/gabor-boros/minutes/internal/pkg/client"
	"github.com/stretchr/testify/require"
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

type mockServerOpts struct {
	Path        string
	Method      string
	StatusCode  int
	Username    string
	Password    string
	RequestData interface{}
	Token       string
	TokenHeader string
}

func mockServer(t *testing.T, e *mockServerOpts) *httptest.Server {
	return httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		require.Equal(t, e.Method, r.Method, "API call methods are not matching")
		require.Equal(t, e.Path, r.URL.Path, "API call URLs are not matching")

		if e.Username != "" && e.Password != "" {
			username, password, _ := r.BasicAuth()
			require.Equal(t, e.Username, username, "API call basic auth username mismatch")
			require.Equal(t, e.Password, password, "API call basic auth password mismatch")
		}

		if e.Token != "" {
			headerValue := r.Header.Get(e.TokenHeader)
			require.Equal(t, e.Token, headerValue, "API call auth token mismatch")
		}

		if e.RequestData != nil {
			var data interface{}

			switch dataType := getDataType(e.RequestData); dataType {
			case "*testData":
				data = e.RequestData.(*testData)
			default:
				t.Fatalf("%s is not a known data type", dataType)
			}

			if err := json.NewDecoder(r.Body).Decode(&data); err != nil {
				t.Fatal(err)
			}
		}

		w.WriteHeader(e.StatusCode)
	}))
}

func newMockServer(t *testing.T, opts *mockServerOpts) *httptest.Server {
	mockServer := mockServer(t, opts)
	require.NotNil(t, mockServer, "cannot create mock server")
	return mockServer
}

func TestSendRequest_GET(t *testing.T) {
	mockServer := newMockServer(t, &mockServerOpts{
		Path:       "/endpoint",
		Method:     http.MethodGet,
		StatusCode: http.StatusOK,
	})
	defer mockServer.Close()

	resp, err := client.SendRequest(context.Background(), &client.SendRequestOpts{
		Method: http.MethodGet,
		Path:   "/endpoint",
		ClientOpts: &client.HTTPClientOpts{
			HTTPClient: http.DefaultClient,
			BaseURL:    mockServer.URL,
		},
		Data: nil,
	})

	require.Nil(t, err, "request failed")
	require.Equal(t, http.StatusOK, resp.StatusCode)
}

func TestSendRequest_POST(t *testing.T) {
	data := &testData{
		Message: "expected post request data",
	}

	mockServer := newMockServer(t, &mockServerOpts{
		Path:        "/endpoint",
		Method:      http.MethodPost,
		StatusCode:  http.StatusOK,
		RequestData: data,
	})
	defer mockServer.Close()

	requestOpts := &client.HTTPClientOpts{
		HTTPClient: http.DefaultClient,
		BaseURL:    mockServer.URL,
	}

	resp, err := client.SendRequest(context.Background(), &client.SendRequestOpts{
		Method:     http.MethodPost,
		Path:       "/endpoint",
		ClientOpts: requestOpts,
		Data:       data,
	})

	require.Nil(t, err, "request failed")
	require.Equal(t, http.StatusOK, resp.StatusCode)
}

func TestSendRequest_BasicAuth(t *testing.T) {
	mockServer := newMockServer(t, &mockServerOpts{
		Path:       "/endpoint",
		Method:     http.MethodGet,
		StatusCode: http.StatusOK,
		Username:   "Thor",
		Password:   "The strongest Avenger",
	})
	defer mockServer.Close()

	resp, err := client.SendRequest(context.Background(), &client.SendRequestOpts{
		Method: http.MethodGet,
		Path:   "/endpoint",
		ClientOpts: &client.HTTPClientOpts{
			HTTPClient: http.DefaultClient,
			BaseURL:    mockServer.URL,
			Username:   "Thor",
			Password:   "The strongest Avenger",
		},
		Data: nil,
	})

	require.Nil(t, err, "request failed")
	require.Equal(t, http.StatusOK, resp.StatusCode)
}

func TestSendRequest_TokenAuth(t *testing.T) {
	mockServer := newMockServer(t, &mockServerOpts{
		Path:        "/endpoint",
		Method:      http.MethodGet,
		StatusCode:  http.StatusOK,
		Token:       "t-o-k-e-n",
		TokenHeader: "X-API-Token",
	})
	defer mockServer.Close()

	resp, err := client.SendRequest(context.Background(), &client.SendRequestOpts{
		Method: http.MethodGet,
		Path:   "/endpoint",
		ClientOpts: &client.HTTPClientOpts{
			HTTPClient:  http.DefaultClient,
			BaseURL:     mockServer.URL,
			Token:       "t-o-k-e-n",
			TokenHeader: "X-API-Token",
		},
		Data: nil,
	})

	require.Nil(t, err, "request failed")
	require.Equal(t, http.StatusOK, resp.StatusCode)
}

func TestSendRequest_TokenAuth_NoHeader(t *testing.T) {
	mockServer := newMockServer(t, &mockServerOpts{
		Path:       "/endpoint",
		Method:     http.MethodGet,
		StatusCode: http.StatusOK,
	})
	defer mockServer.Close()

	resp, err := client.SendRequest(context.Background(), &client.SendRequestOpts{
		Method: http.MethodGet,
		Path:   "/endpoint",
		ClientOpts: &client.HTTPClientOpts{
			HTTPClient:  http.DefaultClient,
			BaseURL:     mockServer.URL,
			Token:       "t-o-k-e-n",
			TokenHeader: "",
		},
		Data: nil,
	})

	require.Nil(t, resp, "request unexpectedly sent")
	require.NotNil(t, err, "request unexpectedly succeeded")
}

func TestSendRequest_Error(t *testing.T) {
	mockServer := newMockServer(t, &mockServerOpts{
		Path:       "/endpoint",
		Method:     http.MethodGet,
		StatusCode: http.StatusInternalServerError,
	})
	defer mockServer.Close()

	resp, err := client.SendRequest(context.Background(), &client.SendRequestOpts{
		Method: http.MethodGet,
		Path:   "/endpoint",
		ClientOpts: &client.HTTPClientOpts{
			HTTPClient: http.DefaultClient,
			BaseURL:    mockServer.URL,
		},
		Data: nil,
	})

	require.Nil(t, resp, "response unexpectedly succeeded")
	require.NotNil(t, err, "response unexpectedly succeeded")
}
