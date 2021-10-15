package toggl

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"regexp"
	"strconv"
	"time"

	"github.com/gabor-boros/minutes/internal/pkg/client"
	"github.com/gabor-boros/minutes/internal/pkg/worklog"
)

const (
	// DateFormat is the ISO 8601 format used by Toggl to parse time.
	DateFormat string = "2006-01-02"
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

// PaginatedResponse represents the response of Toggl Track report APIs.
// The response would have more fields, but those are not relevant for us.
type PaginatedResponse struct {
	TotalCount int          `json:"total_count"`
	PerPage    int          `json:"per_page"`
	Data       []FetchEntry `json:"data"`
}

// WorklogSearchParams represents the parameters used to filter search results.
type WorklogSearchParams struct {
	Since       string
	Until       string
	Page        int
	WorkspaceID int
}

// ClientOpts is the client specific options, extending client.BaseClientOpts.
type ClientOpts struct {
	client.BaseClientOpts
	Workspace int
}

type togglClient struct {
	opts *ClientOpts
}

func (c *togglClient) getSearchURL(params *WorklogSearchParams) (string, error) {
	worklogURL, err := url.Parse(c.opts.BaseURL + PathWorklog)
	if err != nil {
		return "", err
	}

	queryParams := worklogURL.Query()
	queryParams.Add("since", params.Since)
	queryParams.Add("until", params.Until)
	queryParams.Add("page", strconv.Itoa(params.Page))
	queryParams.Add("workspace_id", strconv.Itoa(params.WorkspaceID))
	queryParams.Add("user_agent", "github.com/gabor-boros/minutes")
	worklogURL.RawQuery = queryParams.Encode()

	return fmt.Sprintf("%s?%s", worklogURL.Path, worklogURL.Query().Encode()), nil
}

func (c *togglClient) parseEntries(fetchedEntries []FetchEntry, tagsAsTasksRegex *regexp.Regexp) ([]worklog.Entry, error) {
	var entries []worklog.Entry

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

		if c.opts.TagsAsTasks && len(fetchedEntry.Tags) > 0 {
			var tags []worklog.IDNameField
			for _, tag := range fetchedEntry.Tags {
				tags = append(tags, worklog.IDNameField{
					ID:   tag,
					Name: tag,
				})
			}

			splitEntries := entry.SplitByTagsAsTasks(entry.Summary, tagsAsTasksRegex, tags)
			entries = append(entries, splitEntries...)
		} else {
			entries = append(entries, entry)
		}
	}

	return entries, nil
}

func (c *togglClient) FetchEntries(ctx context.Context, opts *client.FetchOpts) ([]worklog.Entry, error) {
	var err error
	var entries []worklog.Entry
	var tagsAsTasksRegex *regexp.Regexp

	if c.opts.TagsAsTasks {
		tagsAsTasksRegex, err = regexp.Compile(c.opts.TagsAsTasksRegex)
		if err != nil {
			return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
		}
	}

	var pageSize int
	currentPage := 1
	paginationNeeded := true

	for paginationNeeded {
		searchParams := &WorklogSearchParams{
			Since:       opts.Start.Format(DateFormat),
			Until:       opts.End.Format(DateFormat),
			Page:        currentPage,
			WorkspaceID: c.opts.Workspace,
		}

		searchURL, err := c.getSearchURL(searchParams)
		if err != nil {
			return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
		}

		resp, err := client.SendRequest(ctx, http.MethodGet, searchURL, nil, &c.opts.HTTPClientOptions)
		if err != nil {
			return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
		}

		var paginatedResponse PaginatedResponse
		if err = json.NewDecoder(resp.Body).Decode(&paginatedResponse); err != nil {
			return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
		}

		parsedEntries, err := c.parseEntries(paginatedResponse.Data, tagsAsTasksRegex)
		if err != nil {
			return nil, err
		}

		entries = append(entries, parsedEntries...)

		pageSize = paginatedResponse.PerPage
		paginationNeeded = (paginatedResponse.TotalCount - pageSize*currentPage) > 0
		currentPage++
	}

	return entries, nil
}

// NewClient returns a new Toggl client.
func NewClient(opts *ClientOpts) client.Fetcher {
	return &togglClient{
		opts: opts,
	}
}
