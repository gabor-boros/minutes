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
	Workspace string
}

type clockifyClient struct {
	opts *ClientOpts
}

func (c *clockifyClient) getSearchURL(user string, params *WorklogSearchParams) (string, error) {
	searchPath := fmt.Sprintf(PathWorklog, c.opts.Workspace, user)
	worklogURL, err := url.Parse(c.opts.BaseURL + searchPath)
	if err != nil {
		return "", err
	}

	queryParams := worklogURL.Query()
	queryParams.Add("start", params.Start)
	queryParams.Add("end", params.End)
	queryParams.Add("page", strconv.Itoa(params.Page))
	queryParams.Add("page-size", strconv.Itoa(params.PageSize))
	queryParams.Add("hydrated", strconv.FormatBool(params.Hydrated))
	queryParams.Add("in-progress", strconv.FormatBool(params.InProgress))
	worklogURL.RawQuery = queryParams.Encode()

	return fmt.Sprintf("%s?%s", worklogURL.Path, worklogURL.Query().Encode()), nil
}

func (c *clockifyClient) fetchEntries(ctx context.Context, path string) ([]FetchEntry, error) {
	resp, err := client.SendRequest(ctx, &client.SendRequestOpts{
		Method:     http.MethodGet,
		Path:       path,
		ClientOpts: &c.opts.HTTPClientOpts,
		Data:       nil,
	})

	if err != nil {
		return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
	}

	var fetchedEntries []FetchEntry
	if err = json.NewDecoder(resp.Body).Decode(&fetchedEntries); err != nil {
		return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
	}

	return fetchedEntries, err
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

		if c.opts.TagsAsTasks && len(entry.Tags) > 0 {
			pageEntries := worklogEntry.SplitByTagsAsTasks(entry.Description, tagsAsTasksRegex, entry.Tags)
			entries = append(entries, pageEntries...)
		} else {
			entries = append(entries, worklogEntry)
		}
	}

	return entries
}

func (c *clockifyClient) FetchEntries(ctx context.Context, opts *client.FetchOpts) (worklog.Entries, error) {
	var err error
	var entries worklog.Entries
	currentPage := 1
	pageSize := 100

	var tagsAsTasksRegex *regexp.Regexp
	if c.opts.TagsAsTasks {
		tagsAsTasksRegex, err = regexp.Compile(c.opts.TagsAsTasksRegex)
		if err != nil {
			return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
		}
	}

	// Naive pagination as the API does not return the number of total entries
	for currentPage*pageSize < MaxPageLength {
		searchParams := &WorklogSearchParams{
			Start:      utils.DateFormatRFC3339.Format(opts.Start.Local()),
			End:        utils.DateFormatRFC3339.Format(opts.End.Local()),
			Page:       currentPage,
			PageSize:   pageSize,
			Hydrated:   true,
			InProgress: false,
		}

		searchURL, err := c.getSearchURL(opts.User, searchParams)
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

// NewClient returns a new Clockify client.
func NewClient(opts *ClientOpts) client.Fetcher {
	return &clockifyClient{
		opts: opts,
	}
}
