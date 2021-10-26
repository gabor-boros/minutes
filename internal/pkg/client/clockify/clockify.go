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
}

func (c *clockifyClient) parseEntries(fetchedEntries []FetchEntry, tagsAsTasksRegex *regexp.Regexp) worklog.Entries {
	var entries worklog.Entries

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
			pageEntries := worklogEntry.SplitByTagsAsTasks(entry.Description, tagsAsTasksRegex, entry.Tags)
			entries = append(entries, pageEntries...)
		} else {
			entries = append(entries, worklogEntry)
		}
	}

	return entries
}

func (c *clockifyClient) fetchEntries(ctx context.Context, searchURL string) ([]FetchEntry, error) {
	resp, err := c.Call(ctx, &client.HTTPRequestOpts{
		Method:  http.MethodGet,
		Url:     searchURL,
		Auth:    c.authenticator,
		Timeout: c.Timeout,
	})

	if err != nil {
		return nil, err
	}

	var fetchedEntries []FetchEntry
	if err = json.Unmarshal(resp, &fetchedEntries); err != nil {
		return nil, err
	}

	return fetchedEntries, nil
}

func (c *clockifyClient) FetchEntries(ctx context.Context, opts *client.FetchOpts) (worklog.Entries, error) {
	var err error
	var entries worklog.Entries
	currentPage := 1
	pageSize := 100

	var tagsAsTasksRegex *regexp.Regexp
	if c.TagsAsTasks {
		tagsAsTasksRegex, err = regexp.Compile(c.TagsAsTasksRegex)
		if err != nil {
			return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
		}
	}

	// Naive pagination as the API does not return the number of total entries
	for currentPage*pageSize < MaxPageLength {
		searchURL, err := c.URL(fmt.Sprintf(PathWorklog, c.workspace, opts.User), map[string]string{
			"start":       utils.DateFormatRFC3339UTC.Format(opts.Start.Local()),
			"end":         utils.DateFormatRFC3339UTC.Format(opts.End.Local()),
			"page":        strconv.Itoa(currentPage),
			"page-size":   strconv.Itoa(pageSize),
			"hydrated":    strconv.FormatBool(true),
			"in-progress": strconv.FormatBool(false),
		})
		if err != nil {
			return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
		}

		fetchedEntries, err := c.fetchEntries(ctx, searchURL)
		if err != nil {
			return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
		}

		// The API returned no entries, meaning no entries left
		if len(fetchedEntries) == 0 {
			break
		}

		entries = append(entries, c.parseEntries(fetchedEntries, tagsAsTasksRegex)...)
		currentPage++
	}

	return entries, nil
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

	return &clockifyClient{
		authenticator:  authenticator,
		HTTPClient:     &client.HTTPClient{BaseURL: baseURL},
		BaseClientOpts: &opts.BaseClientOpts,
		workspace:      opts.Workspace,
	}, nil
}
