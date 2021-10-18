package client

import (
	"bytes"
	"context"
	"encoding/json"
	"errors"
	"fmt"
	"io"
	"net/http"
	"net/url"
	"time"

	"github.com/jedib0t/go-pretty/v6/progress"

	"github.com/gabor-boros/minutes/internal/pkg/worklog"
)

var (
	// ErrFetchEntries wraps the error when fetch failed.
	ErrFetchEntries = errors.New("failed to fetch entries")
	// ErrUploadEntries wraps the error when upload failed.
	ErrUploadEntries = errors.New("failed to upload entries")
)

// HTTPClientOpts specifies all options that are required for HTTP clients.
type HTTPClientOpts struct {
	HTTPClient *http.Client
	// BaseURL for the API, without a trailing slash.
	BaseURL string
	// Username used for authentication.
	Username string
	// Password used for authentication.
	//
	// If both Password and Token are set, Token takes precedence.
	Password string
	// Token is the API token used by the source our target API.
	//
	// If Token is set, TokenHeader must not be empty.
	// If both Password and Token are set, Token takes precedence.
	Token string
	// TokenHeader is the header name that contains the auth token.
	TokenHeader string
}

// BaseClientOpts specifies the common options the clients are using.
// When a client needs other options as well, it composes a new set of options
// using BaseClientOpts.
type BaseClientOpts struct {
	HTTPClientOpts
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
}

// FetchOpts specifies the only options for Fetchers.
// In contract to the BaseClientOpts, these options shall not be extended or
// overridden.
type FetchOpts struct {
	User  string
	Start time.Time
	End   time.Time
}

// Fetcher specifies the functions used to fetch worklog entries.
type Fetcher interface {
	// FetchEntries from a given source and return the list of worklog entries
	// If the fetching resulted in an error, the list of worklog entries will be
	// nil and an error will return.
	FetchEntries(ctx context.Context, opts *FetchOpts) ([]worklog.Entry, error)
}

// UploadOpts specifies the only options for the Uploader. In contrast to the
// BaseClientOpts, these options shall not be extended or overridden.
type UploadOpts struct {
	// RoundToClosestMinute indicates to round the billed and unbilled duration
	// separately to the closest minute.
	// If the elapsed time is 30 seconds or more, the closest minute is the
	// next minute, otherwise the previous one. In case the previous minute is
	// 0 (zero), then 0 (zero) will be used for the billed and/or unbilled
	// duration.
	RoundToClosestMinute bool
	// TreatDurationAsBilled indicates to use every time spent as billed.
	TreatDurationAsBilled bool
	// CreateMissingResources indicates the need of resource creation if the
	// resource is missing.
	// In the case of some Uploader, the resources must exist to be able to
	// use them by their ID or name.
	CreateMissingResources bool
	// User represents the user in which name the time log will be uploaded.
	User string
	// ProgressWriter represents a writer that tracks the upload progress.
	// In case the ProgressWriter is nil, that means the upload progress should
	// not be tracked, hence, that's not an error.
	ProgressWriter progress.Writer
}

// Uploader specifies the functions used to upload worklog entries.
type Uploader interface {
	// UploadEntries to a given target.
	// If the upload resulted in an error, the upload will stop and an error
	// will return.
	UploadEntries(ctx context.Context, entries []worklog.Entry, errChan chan error, opts *UploadOpts)
}

// FetchUploader is the combination of Fetcher and Uploader.
// The FetchUploader can to fetch entries from and upload to a given resource.
type FetchUploader interface {
	Fetcher
	Uploader
}

// SendRequestOpts represents the parameters needed for sending a request.
// Since SendRequest is for sending requests to HTTP based APIs, it receives
// the HTTPClientOpts as well for its options.
type SendRequestOpts struct {
	Method     string
	Path       string
	ClientOpts *HTTPClientOpts
	Data       interface{}
}

// SendRequest is a helper for any Fetcher and Uploader that must APIs.
// The SendRequest function prepares a new HTTP request, sends it and returns
// the response for further parsing. If the response status is not 200 or 201,
// the function returns an error.
func SendRequest(ctx context.Context, opts *SendRequestOpts) (*http.Response, error) {
	var err error
	var marshalledData []byte

	requestURL, err := url.Parse(opts.ClientOpts.BaseURL + opts.Path)
	if err != nil {
		return nil, err
	}

	if opts.Data != nil {
		marshalledData, err = json.Marshal(opts.Data)
		if err != nil {
			return nil, err
		}
	}

	req, err := http.NewRequestWithContext(ctx, opts.Method, requestURL.String(), bytes.NewBuffer(marshalledData))
	if err != nil {
		return nil, err
	}

	req.Header.Add("Content-Type", "application/json")

	if opts.ClientOpts.Token != "" {
		if opts.ClientOpts.TokenHeader == "" {
			return nil, errors.New("no token header name")
		}

		req.Header.Add(opts.ClientOpts.TokenHeader, opts.ClientOpts.Token)
	} else {
		req.SetBasicAuth(opts.ClientOpts.Username, opts.ClientOpts.Password)
	}

	resp, err := opts.ClientOpts.HTTPClient.Do(req)
	if err != nil {
		return nil, err
	}

	if !(resp.StatusCode == http.StatusOK || resp.StatusCode == http.StatusCreated) {
		errBody, err := io.ReadAll(resp.Body)
		if err != nil {
			return nil, err
		}

		return nil, fmt.Errorf("%d: %s", resp.StatusCode, string(errBody))
	}

	return resp, err
}
