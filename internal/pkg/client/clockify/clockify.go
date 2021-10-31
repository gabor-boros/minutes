package clockify

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"regexp"
	"time"

	"strconv"

	"github.com/gabor-boros/minutes/internal/pkg/client"
	"github.com/gabor-boros/minutes/internal/pkg/utils"
	"github.com/gabor-boros/minutes/internal/pkg/worklog"
)

const (
	// MaxPageLength is the maximum page length defined by Clockify.
	MaxPageLength int = 5000
	// PathWorklog is the API endpoint used to search and create worklogs.
	PathWorklog string = "/api/v1/workspaces/%s/user/%s/time-entries"
)

// Project represents the project assigned to an entry.
type Project struct {
	worklog.IDNameField
	ClientID   string `json:"clientId"`
	ClientName string `json:"clientName"`
}

// Interval represents the Start and End date of an entry.
type Interval struct {
	Start time.Time `json:"start"`
	End   time.Time `json:"end"`
}

// FetchEntry represents the entry fetched from Clockify.
type FetchEntry struct {
	Description  string                `json:"description"`
	Billable     bool                  `json:"billable"`
	Project      Project               `json:"project"`
	TimeInterval Interval              `json:"timeInterval"`
	Task         worklog.IDNameField   `json:"task"`
	Tags         []worklog.IDNameField `json:"tags"`
}

// WorklogSearchParams represents the parameters used to filter search results.
// Hydrated indicates to return the "expanded" search result. Expanded result
// contains the project, task, and tag details, not just their ID.
type WorklogSearchParams struct {
	Start      string
	End        string
	Page       int
	PageSize   int
	Hydrated   bool
	InProgress bool
}

// ClientOpts is the client specific options, extending client.BaseClientOpts.
type ClientOpts struct {
	client.BaseClientOpts
	client.TokenAuth
	BaseURL   string
	Workspace string
}

type clockifyClient struct {
	*client.BaseClientOpts
	*client.HTTPClient
	authenticator client.Authenticator
	workspace     string

	// TODO: opts.TagsAsTasksRegex should be a regexp to avoid this
	tagsAsTasksRegex *regexp.Regexp
}

func (c *clockifyClient) parseEntries(rawEntries interface{}) (worklog.Entries, error) {
	var entries worklog.Entries

	fetchedEntries, ok := rawEntries.([]FetchEntry)
	if !ok {
		return nil, fmt.Errorf("%v: %s", client.ErrFetchEntries, "cannot parse returned entries")
	}

	for _, entry := range fetchedEntries {
		billableDuration := entry.TimeInterval.End.Sub(entry.TimeInterval.Start)
		unbillableDuration := time.Duration(0)

		if !entry.Billable {
			unbillableDuration = billableDuration
			billableDuration = 0
		}

		worklogEntry := worklog.Entry{
			Client: worklog.IDNameField{
				ID:   entry.Project.ClientID,
				Name: entry.Project.ClientName,
			},
			Project: worklog.IDNameField{
				ID:   entry.Project.ID,
				Name: entry.Project.Name,
			},
			Task: worklog.IDNameField{
				ID:   entry.Task.ID,
				Name: entry.Task.Name,
			},
			Summary:            entry.Task.Name,
			Notes:              entry.Description,
			Start:              entry.TimeInterval.Start,
			BillableDuration:   billableDuration,
			UnbillableDuration: unbillableDuration,
		}

		if c.TagsAsTasks && len(entry.Tags) > 0 {
			pageEntries := worklogEntry.SplitByTagsAsTasks(entry.Description, c.tagsAsTasksRegex, entry.Tags)
			entries = append(entries, pageEntries...)
		} else {
			entries = append(entries, worklogEntry)
		}
	}

	return entries, nil
}

func (c *clockifyClient) fetchEntries(ctx context.Context, reqURL string) (interface{}, *client.PaginatedFetchResponse, error) {
	resp, err := c.Call(ctx, &client.HTTPRequestOpts{
		Method:  http.MethodGet,
		Url:     reqURL,
		Auth:    c.authenticator,
		Timeout: c.Timeout,
	})

	if err != nil {
		return nil, nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
	}

	var fetchedEntries []FetchEntry
	if err = json.Unmarshal(resp, &fetchedEntries); err != nil {
		return nil, nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
	}

	return fetchedEntries, &client.PaginatedFetchResponse{}, err
}

func (c *clockifyClient) FetchEntries(ctx context.Context, opts *client.FetchOpts) (worklog.Entries, error) {
	fetchURL, err := c.URL(fmt.Sprintf(PathWorklog, c.workspace, opts.User), map[string]string{
		"start":       utils.DateFormatRFC3339UTC.Format(opts.Start.Local()),
		"end":         utils.DateFormatRFC3339UTC.Format(opts.End.Local()),
		"hydrated":    strconv.FormatBool(true),
		"in-progress": strconv.FormatBool(false),
	})

	if err != nil {
		return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
	}

	return c.PaginatedFetch(ctx, &client.PaginatedFetchOpts{
		URL:           fetchURL,
		PageSizeParam: "page-size",
		FetchFunc:     c.fetchEntries,
		ParseFunc:     c.parseEntries,
	})
}

// NewFetcher returns a new Clockify client for fetching entries.
func NewFetcher(opts *ClientOpts) (client.Fetcher, error) {
	baseURL, err := url.Parse(opts.BaseURL)
	if err != nil {
		return nil, err
	}

	authenticator, err := client.NewTokenAuth(opts.Header, "", opts.Token)
	if err != nil {
		return nil, err
	}

	// TODO: Remove this after opt.TagsAsTasksRegex is refactored
	var tagsAsTasksRegex *regexp.Regexp
	if opts.TagsAsTasks {
		tagsAsTasksRegex, err = regexp.Compile(opts.TagsAsTasksRegex)
		if err != nil {
			return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
		}
	}

	return &clockifyClient{
		authenticator:  authenticator,
		HTTPClient:     &client.HTTPClient{BaseURL: baseURL},
		BaseClientOpts: &opts.BaseClientOpts,
		workspace:      opts.Workspace,
		// TODO: Remove this after opt.TagsAsTasksRegex is refactored
		tagsAsTasksRegex: tagsAsTasksRegex,
	}, nil
}
