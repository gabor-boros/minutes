package toggl

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"strconv"
	"strings"
	"time"

	"github.com/gabor-boros/minutes/internal/pkg/client"
	"github.com/gabor-boros/minutes/internal/pkg/utils"
	"github.com/gabor-boros/minutes/internal/pkg/worklog"
)

const (
	// PathWorklog is the endpoint used to search existing worklogs.
	PathWorklog string = "/reports/api/v2/details"
)

// FetchEntry represents the entry fetched from Toggl Track.
type FetchEntry struct {
	Client      string    `json:"client"`
	Description string    `json:"description"`
	Duration    int       `json:"dur"`
	IsBillable  bool      `json:"is_billable"`
	Project     string    `json:"project"`
	ProjectID   int       `json:"pid"`
	Start       time.Time `json:"start"`
	End         time.Time `json:"end"`
	Tags        []string  `json:"tags"`
	Task        string    `json:"task"`
	TaskID      int       `json:"tid"`
}

// FetchResponse represents the response of Toggl Track report APIs.
// The response would have more fields, but those are not relevant for us.
type FetchResponse struct {
	TotalCount int          `json:"total_count"`
	PerPage    int          `json:"per_page"`
	Data       []FetchEntry `json:"data"`
}

// ClientOpts is the client specific options, extending client.BaseClientOpts.
type ClientOpts struct {
	client.BaseClientOpts
	client.BasicAuth
	BaseURL   string
	Workspace int
}

type togglClient struct {
	*client.BaseClientOpts
	*client.HTTPClient
	authenticator client.Authenticator
	workspace     int
}

func (c *togglClient) parseEntries(rawEntries interface{}) (worklog.Entries, error) {
	var entries worklog.Entries

	fetchedEntries, ok := rawEntries.([]FetchEntry)
	if !ok {
		return nil, fmt.Errorf("%v: %s", client.ErrFetchEntries, "cannot parse returned entries")
	}

	for _, fetchedEntry := range fetchedEntries {
		billableDuration := time.Millisecond * time.Duration(fetchedEntry.Duration)
		unbillableDuration := time.Duration(0)

		if !fetchedEntry.IsBillable {
			unbillableDuration = billableDuration
			billableDuration = 0
		}

		entry := worklog.Entry{
			Client: worklog.IDNameField{
				ID:   fetchedEntry.Client,
				Name: fetchedEntry.Client,
			},
			Project: worklog.IDNameField{
				ID:   strconv.Itoa(fetchedEntry.ProjectID),
				Name: fetchedEntry.Project,
			},
			Task: worklog.IDNameField{
				ID:   strconv.Itoa(fetchedEntry.TaskID),
				Name: fetchedEntry.Task,
			},
			Summary:            fetchedEntry.Description,
			Notes:              fetchedEntry.Description,
			Start:              fetchedEntry.Start,
			BillableDuration:   billableDuration,
			UnbillableDuration: unbillableDuration,
		}

		if c.TagsAsTasks && len(fetchedEntry.Tags) > 0 {
			var tags []worklog.IDNameField
			for _, tag := range fetchedEntry.Tags {
				tags = append(tags, worklog.IDNameField{
					ID:   tag,
					Name: tag,
				})
			}

			splitEntries := entry.SplitByTagsAsTasks(entry.Summary, c.TagsAsTasksRegex, tags)
			entries = append(entries, splitEntries...)
		} else {
			entries = append(entries, entry)
		}
	}

	return entries, nil
}

func (c *togglClient) fetchEntries(ctx context.Context, reqURL string) (interface{}, *client.PaginatedFetchResponse, error) {
	resp, err := c.Call(ctx, &client.HTTPRequestOpts{
		Method:  http.MethodGet,
		Url:     reqURL,
		Auth:    c.authenticator,
		Timeout: c.Timeout,
	})

	if err != nil {
		return nil, nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
	}

	var fetchResponse FetchResponse
	if err = json.Unmarshal(resp, &fetchResponse); err != nil {
		return nil, nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
	}

	paginatedResponse := &client.PaginatedFetchResponse{
		EntriesPerPage: fetchResponse.PerPage,
		TotalEntries:   fetchResponse.TotalCount,
	}

	return fetchResponse.Data, paginatedResponse, err
}

func (c *togglClient) FetchEntries(ctx context.Context, opts *client.FetchOpts) (worklog.Entries, error) {
	userID, err := strconv.Atoi(strings.Split(opts.User, ",")[0])
	if err != nil {
		return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
	}

	fetchURL, err := c.URL(PathWorklog, map[string]string{
		"since":        utils.DateFormatISO8601.Format(opts.Start),
		"until":        utils.DateFormatISO8601.Format(opts.End),
		"user_id":      strconv.Itoa(userID),
		"workspace_id": strconv.Itoa(c.workspace),
		"user_agent":   "github.com/gabor-boros/minutes",
	})

	if err != nil {
		return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
	}

	return c.PaginatedFetch(ctx, &client.PaginatedFetchOpts{
		URL:       fetchURL,
		FetchFunc: c.fetchEntries,
		ParseFunc: c.parseEntries,
	})
}

// NewFetcher returns a new Toggl client for fetching entries.
func NewFetcher(opts *ClientOpts) (client.Fetcher, error) {
	baseURL, err := url.Parse(opts.BaseURL)
	if err != nil {
		return nil, err
	}

	authenticator, err := client.NewBasicAuth(opts.Username, opts.Password)
	if err != nil {
		return nil, err
	}

	return &togglClient{
		authenticator: authenticator,
		HTTPClient: &client.HTTPClient{
			BaseURL: baseURL,
		},
		BaseClientOpts: &opts.BaseClientOpts,
		workspace:      opts.Workspace,
	}, nil
}
