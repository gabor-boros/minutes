package harvest

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"net/url"
	"strconv"
	"time"

	"github.com/gabor-boros/minutes/internal/pkg/client"
	"github.com/gabor-boros/minutes/internal/pkg/utils"
	"github.com/gabor-boros/minutes/internal/pkg/worklog"
)

const (
	// PathWorklog is the endpoint used to search existing worklogs.
	PathWorklog string = "/v2/time_entries"
)

// FetchEntry represents the entry fetched from Harvest.
type FetchEntry struct {
	Client    worklog.IntIDNameField `json:"client"`
	Project   worklog.IntIDNameField `json:"project"`
	Task      worklog.IntIDNameField `json:"task"`
	Notes     string                 `json:"notes"`
	SpentDate string                 `json:"spent_date"`
	Hours     float32                `json:"hours"`
	CreatedAt time.Time              `json:"created_at"`
	Billable  bool                   `json:"billable"`
	IsRunning bool                   `json:"is_running"`
}

// Start returns the start date created from the spent date and created at.
// The spent date represents the date the user wants the entry to be logged,
// e.g: 2021-10-01. The creation date represents the actual creation of the
// entry, e.g: 2021-10-02T10:26:20Z. Since Harvest is not precise with the
// spent date, we have to create a start date from these two entries. This is
// needed, because if the user is manually creating an entry, and creates on
// a wrong date accidentally, after editing the entry, the spent date will be
// updated, though the creation date not.
func (e *FetchEntry) Start() (time.Time, error) {
	spentDate, err := utils.DateFormatISO8601.Parse(e.SpentDate)
	if err != nil {
		return time.Time{}, err
	}

	return time.Date(
		spentDate.Year(),
		spentDate.Month(),
		spentDate.Day(),
		e.CreatedAt.Hour(),
		e.CreatedAt.Minute(),
		e.CreatedAt.Second(),
		e.CreatedAt.Nanosecond(),
		e.CreatedAt.Location(),
	), nil
}

// FetchResponse represents the relevant response data.
// Although the response contains a lot more information about pagination, it
// cannot be used with the current structure.
type FetchResponse struct {
	TimeEntries  []FetchEntry `json:"time_entries"`
	PerPage      int          `json:"per_page"`
	TotalEntries int          `json:"total_entries"`
}

// ClientOpts is the client specific options, extending client.BaseClientOpts.
type ClientOpts struct {
	client.BaseClientOpts
	client.TokenAuth
	BaseURL string
	Account int
}

type harvestClient struct {
	*client.BaseClientOpts
	*client.HTTPClient
	authenticator client.Authenticator
	account       int
}

func (c *harvestClient) parseEntries(rawEntries interface{}) (worklog.Entries, error) {
	var entries worklog.Entries

	fetchedEntries, ok := rawEntries.([]FetchEntry)
	if !ok {
		return nil, fmt.Errorf("%v: %s", client.ErrFetchEntries, "cannot parse returned entries")
	}

	for _, fetchedEntry := range fetchedEntries {
		startDate, err := fetchedEntry.Start()
		if err != nil {
			return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
		}

		billableDuration, err := time.ParseDuration(fmt.Sprintf("%fh", fetchedEntry.Hours))
		if err != nil {
			return nil, fmt.Errorf("%v: %v", client.ErrFetchEntries, err)
		}

		unbillableDuration := time.Duration(0)

		if !fetchedEntry.Billable {
			unbillableDuration = billableDuration
			billableDuration = 0
		}

		entries = append(entries, worklog.Entry{
			Client:             fetchedEntry.Client.ConvertToIDNameField(),
			Project:            fetchedEntry.Project.ConvertToIDNameField(),
			Task:               fetchedEntry.Task.ConvertToIDNameField(),
			Summary:            fetchedEntry.Notes,
			Notes:              fetchedEntry.Notes,
			Start:              startDate,
			BillableDuration:   billableDuration,
			UnbillableDuration: unbillableDuration,
		})
	}

	return entries, nil
}

func (c *harvestClient) fetchEntries(ctx context.Context, reqURL string) (interface{}, *client.PaginatedFetchResponse, error) {
	resp, err := c.Call(ctx, &client.HTTPRequestOpts{
		Method:  http.MethodGet,
		Url:     reqURL,
		Auth:    c.authenticator,
		Timeout: c.Timeout,
		Headers: map[string]string{
			"Harvest-Account-ID": strconv.Itoa(c.account),
		},
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
		TotalEntries:   fetchResponse.TotalEntries,
	}

	return fetchResponse.TimeEntries, paginatedResponse, err
}

func (c *harvestClient) FetchEntries(ctx context.Context, opts *client.FetchOpts) (worklog.Entries, error) {
	fetchURL, err := c.URL(PathWorklog, map[string]string{
		"from":       utils.DateFormatRFC3339UTC.Format(opts.Start),
		"to":         utils.DateFormatRFC3339UTC.Format(opts.End),
		"user_id":    opts.User,
		"is_running": strconv.FormatBool(false),
		"user_agent": "github.com/gabor-boros/minutes",
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

// NewFetcher returns a new Clockify client for fetching entries.
func NewFetcher(opts *ClientOpts) (client.Fetcher, error) {
	baseURL, err := url.Parse(opts.BaseURL)
	if err != nil {
		return nil, err
	}

	authenticator, err := client.NewTokenAuth(opts.Header, opts.TokenName, opts.Token)
	if err != nil {
		return nil, err
	}

	return &harvestClient{
		BaseClientOpts: &opts.BaseClientOpts,
		HTTPClient: &client.HTTPClient{
			BaseURL: baseURL,
		},
		authenticator: authenticator,
		account:       opts.Account,
	}, nil
}
