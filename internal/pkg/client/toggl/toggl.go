package toggl

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"regexp"
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
	UserID      int
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
	queryParams.Add("user_id", strconv.Itoa(params.UserID))
	queryParams.Add("workspace_id", strconv.Itoa(params.WorkspaceID))
	queryParams.Add("user_agent", "github.com/gabor-boros/minutes")
	worklogURL.RawQuery = queryParams.Encode()

	return fmt.Sprintf("%s?%s", worklogURL.Path, worklogURL.Query().Encode()), nil
}

func (c *togglClient) parseEntries(fetchedEntries []FetchEntry, tagsAsTasksRegex *regexp.Regexp) (worklog.Entries, error) {
	var entries worklog.Entries

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

func (c *togglClient) fetchEntries(ctx context.Context, path string, tagsAsTasksRegex *regexp.Regexp) (worklog.Entries, *PaginatedResponse, error) {
	resp, err := client.SendRequest(ctx, &client.SendRequestOpts{
		Method:     http.MethodGet,
		Path:       path,
		ClientOpts: &c.opts.HTTPClientOpts,
		Data:       nil,
	})

	if err != nil {
		return nil, nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
	}

	var paginatedResponse PaginatedResponse
	if err = json.NewDecoder(resp.Body).Decode(&paginatedResponse); err != nil {
		return nil, nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
	}

	parsedEntries, err := c.parseEntries(paginatedResponse.Data, tagsAsTasksRegex)
	if err != nil {
		return nil, nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
	}

	return parsedEntries, &paginatedResponse, err
}

func (c *togglClient) FetchEntries(ctx context.Context, opts *client.FetchOpts) (worklog.Entries, error) {
	var err error
	var entries worklog.Entries
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

	userID, err := strconv.Atoi(strings.Split(opts.User, ",")[0])
	if err != nil {
		return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
	}

	for paginationNeeded {
		searchParams := &WorklogSearchParams{
			Since:       utils.DateFormatISO8601.Format(opts.Start),
			Until:       utils.DateFormatISO8601.Format(opts.End),
			Page:        currentPage,
			UserID:      userID,
			WorkspaceID: c.opts.Workspace,
		}

		searchURL, err := c.getSearchURL(searchParams)
		if err != nil {
			return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
		}

		parsedEntries, paginatedResponse, err := c.fetchEntries(ctx, searchURL, tagsAsTasksRegex)
		if err != nil {
			return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
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
