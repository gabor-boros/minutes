package clockify

import (
	"context"
	"encoding/json"
	"fmt"
	"math"
	"net/http"
	"net/url"
	"regexp"
	"time"

	"strconv"

	"github.com/gabor-boros/minutes/internal/pkg/client"
	"github.com/gabor-boros/minutes/internal/pkg/worklog"
)

const (
	// DateFormat is the specific format used by Clockify to parse time.
	DateFormat string = "2006-01-02T15:04:05Z"
	// MaxPageLength is the maximum page length defined by Clockify.
	MaxPageLength int = 5000
	// PathWorklog is the API endpoint used to search and create worklogs.
	PathWorklog string = "/api/v1/workspaces/%s/user/%s/time-entries"
)

// Project represents the project assigned to an entry.
type Project struct {
	ID         string `json:"id"`
	Name       string `json:"name"`
	ClientID   string `json:"clientId"`
	ClientName string `json:"clientName"`
}

// Tag represents a tag assigned to an entry.
type Tag struct {
	ID   string `json:"id"`
	Name string `json:"name"`
}

// Task represents the task assigned to an entry.
type Task struct {
	ID   string `json:"id"`
	Name string `json:"name"`
}

// Interval represents the Start and End date of an entry.
type Interval struct {
	Start time.Time `json:"start"`
	End   time.Time `json:"end"`
}

// FetchEntry represents the entry fetched from Clockify.
type FetchEntry struct {
	Description  string   `json:"description"`
	Billable     bool     `json:"billable"`
	Project      Project  `json:"project"`
	TimeInterval Interval `json:"timeInterval"`
	Task         Task     `json:"task"`
	Tags         []Tag    `json:"tags"`
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

func (c *clockifyClient) splitEntry(entry FetchEntry, bd time.Duration, ubd time.Duration) (*[]worklog.Entry, error) {
	r, err := regexp.Compile(c.opts.TasksAsTagsRegex)
	if err != nil {
		return nil, err
	}

	tasks := map[string]string{}
	for _, tag := range entry.Tags {
		if task := r.FindString(tag.Name); task != "" {
			tasks[tag.ID] = task
		}
	}

	var items []worklog.Entry
	totalTasks := len(tasks)

	for taskID, taskName := range tasks {
		splitBillableDuration := time.Duration(math.Round(float64(bd.Nanoseconds()) / float64(totalTasks)))
		splitUnbillableDuration := time.Duration(math.Round(float64(ubd.Nanoseconds()) / float64(totalTasks)))

		items = append(items, worklog.Entry{
			Client: worklog.IDNameField{
				ID:   entry.Project.ClientID,
				Name: entry.Project.ClientName,
			},
			Project: worklog.IDNameField{
				ID:   entry.Project.ID,
				Name: entry.Project.Name,
			},
			Task: worklog.IDNameField{
				ID:   taskID,
				Name: taskName,
			},
			Summary:            entry.Description,
			Notes:              entry.Description,
			Start:              entry.TimeInterval.Start,
			BillableDuration:   splitBillableDuration,
			UnbillableDuration: splitUnbillableDuration,
		})
	}

	return &items, nil
}

func (c *clockifyClient) FetchEntries(ctx context.Context, opts *client.FetchOpts) ([]worklog.Entry, error) {
	var items []worklog.Entry
	currentPage := 1
	pageSize := 100

	// Naive pagination as the API does not return the number of total items
	for currentPage*pageSize < MaxPageLength {
		searchParams := &WorklogSearchParams{
			Start:      opts.Start.Format(DateFormat),
			End:        opts.End.Format(DateFormat),
			Page:       currentPage,
			PageSize:   pageSize,
			Hydrated:   true,
			InProgress: false,
		}

		searchURL, err := c.getSearchURL(opts.User, searchParams)
		if err != nil {
			return nil, err
		}

		resp, err := client.SendRequest(ctx, http.MethodGet, searchURL, nil, &c.opts.HTTPClientOptions)
		if err != nil {
			return nil, err
		}

		var entries []FetchEntry
		if err = json.NewDecoder(resp.Body).Decode(&entries); err != nil {
			return nil, err
		}

		// The API returned no entries, meaning no entries left
		if len(entries) == 0 {
			break
		}

		for _, entry := range entries {
			billableDuration := entry.TimeInterval.End.Sub(entry.TimeInterval.Start)
			unbillableDuration := time.Duration(0)

			if !entry.Billable {
				unbillableDuration = billableDuration
				billableDuration = 0
			}

			if c.opts.TasksAsTags && len(entry.Tags) > 0 {
				pageItems, err := c.splitEntry(entry, billableDuration, unbillableDuration)
				if err != nil {
					return nil, err
				}

				items = append(items, *pageItems...)
			} else {
				items = append(items, worklog.Entry{
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
				})
			}
		}

		currentPage++
	}

	return items, nil
}

// NewClient returns a new Clockify client.
func NewClient(opts *ClientOpts) client.Fetcher {
	return &clockifyClient{
		opts: opts,
	}
}
