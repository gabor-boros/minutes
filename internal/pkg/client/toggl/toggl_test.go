package toggl_test

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"net/url"
	"regexp"
	"strconv"
	"testing"
	"time"

	"github.com/gabor-boros/minutes/internal/pkg/client"
	"github.com/gabor-boros/minutes/internal/pkg/client/toggl"
	"github.com/gabor-boros/minutes/internal/pkg/utils"
	"github.com/gabor-boros/minutes/internal/pkg/worklog"
	"github.com/stretchr/testify/require"
)

type mockServerOpts struct {
	Path         string
	QueryParams  url.Values
	Method       string
	StatusCode   int
	Username     string
	Password     string
	ResponseData *toggl.FetchResponse
}

func mockServer(t *testing.T, e *mockServerOpts) *httptest.Server {
	return httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		require.Equal(t, e.Method, r.Method, "API call methods are not matching")
		require.Equal(t, e.Path, r.URL.Path, "API call URLs are not matching")
		require.Equal(t, e.QueryParams, r.URL.Query())

		if e.Username != "" && e.Password != "" {
			username, password, _ := r.BasicAuth()
			require.Equal(t, e.Username, username, "API call basic auth username mismatch")
			require.Equal(t, e.Password, password, "API call basic auth password mismatch")
		}

		if e.ResponseData != nil {
			err := json.NewEncoder(w).Encode(e.ResponseData)
			require.Nil(t, err, "cannot encode response data")
		}

		w.WriteHeader(e.StatusCode)
	}))
}

func newMockServer(t *testing.T, opts *mockServerOpts) *httptest.Server {
	mockServer := mockServer(t, opts)
	require.NotNil(t, mockServer, "cannot create mock server")
	return mockServer
}

func TestTogglClient_FetchEntries(t *testing.T) {
	start := time.Date(2021, 10, 2, 0, 0, 0, 0, time.UTC)
	end := time.Date(2021, 10, 2, 23, 59, 59, 0, time.UTC)

	clientUsername := "token-of-the-day"
	clientPassword := "api_token"

	expectedEntries := worklog.Entries{
		{
			Client: worklog.IDNameField{
				ID:   "My Awesome Company",
				Name: "My Awesome Company",
			},
			Project: worklog.IDNameField{
				ID:   strconv.Itoa(456),
				Name: "MARVEL",
			},
			Task: worklog.IDNameField{
				ID:   strconv.Itoa(789),
				Name: "CPT-2014",
			},
			Summary:            "I met with The Winter Soldier",
			Notes:              "I met with The Winter Soldier",
			Start:              start,
			BillableDuration:   time.Second * 3600,
			UnbillableDuration: 0,
		},
		{
			Client: worklog.IDNameField{
				ID:   "My Awesome Company",
				Name: "My Awesome Company",
			},
			Project: worklog.IDNameField{
				ID:   strconv.Itoa(456),
				Name: "MARVEL",
			},
			Task: worklog.IDNameField{
				ID:   strconv.Itoa(789),
				Name: "CPT-2014",
			},
			Summary:            "I helped him to get back on track",
			Notes:              "I helped him to get back on track",
			Start:              start,
			BillableDuration:   0,
			UnbillableDuration: time.Second * 3600,
		},
	}

	mockServer := newMockServer(t, &mockServerOpts{
		Path: toggl.PathWorklog,
		QueryParams: url.Values{
			"page":         {"1"},
			"per_page":     {"50"},
			"since":        {utils.DateFormatISO8601.Format(start)},
			"until":        {utils.DateFormatISO8601.Format(end)},
			"user_id":      {"987654321"},
			"workspace_id": {"123456789"},
			"user_agent":   {"github.com/gabor-boros/minutes"},
		},
		Method:     http.MethodGet,
		StatusCode: http.StatusOK,
		Username:   clientUsername,
		Password:   clientPassword,
		ResponseData: &toggl.FetchResponse{
			TotalCount: 2,
			PerPage:    50,
			Data: []toggl.FetchEntry{
				{
					Client:      "My Awesome Company",
					Description: "I met with The Winter Soldier",
					Duration:    3600000,
					IsBillable:  true,
					Project:     "MARVEL",
					ProjectID:   456,
					Start:       start,
					End:         start.Add(3600000),
					Tags:        nil,
					Task:        "CPT-2014",
					TaskID:      789,
				},
				{
					Client:      "My Awesome Company",
					Description: "I helped him to get back on track",
					Duration:    3600000,
					IsBillable:  false,
					Project:     "MARVEL",
					ProjectID:   456,
					Start:       start,
					End:         start.Add(3600000),
					Tags:        nil,
					Task:        "CPT-2014",
					TaskID:      789,
				},
			},
		},
	})
	defer mockServer.Close()

	togglClient, err := toggl.NewFetcher(&toggl.ClientOpts{
		BaseClientOpts: client.BaseClientOpts{
			Timeout: client.DefaultRequestTimeout,
		},
		BasicAuth: client.BasicAuth{
			Username: clientUsername,
			Password: clientPassword,
		},
		BaseURL:   mockServer.URL,
		Workspace: 123456789,
	})
	require.Nil(t, err)

	entries, err := togglClient.FetchEntries(context.Background(), &client.FetchOpts{
		User:  "987654321",
		Start: start,
		End:   end,
	})

	require.Nil(t, err, "cannot fetch entries")
	require.ElementsMatch(t, expectedEntries, entries, "fetched entries are not matching")
}

func TestTogglClient_FetchEntries_TagsAsTasks(t *testing.T) {
	start := time.Date(2021, 10, 2, 0, 0, 0, 0, time.UTC)
	end := time.Date(2021, 10, 2, 23, 59, 59, 0, time.UTC)

	clientUsername := "token-of-the-day"
	clientPassword := "api_token"

	expectedEntries := worklog.Entries{
		{
			Client: worklog.IDNameField{
				ID:   "My Awesome Company",
				Name: "My Awesome Company",
			},
			Project: worklog.IDNameField{
				ID:   strconv.Itoa(456),
				Name: "MARVEL",
			},
			Task: worklog.IDNameField{
				ID:   "CPT-2014",
				Name: "CPT-2014",
			},
			Summary:            "I met with The Winter Soldier",
			Notes:              "I met with The Winter Soldier",
			Start:              start,
			BillableDuration:   time.Second * 3600,
			UnbillableDuration: 0,
		},
		{
			Client: worklog.IDNameField{
				ID:   "My Awesome Company",
				Name: "My Awesome Company",
			},
			Project: worklog.IDNameField{
				ID:   strconv.Itoa(456),
				Name: "MARVEL",
			},
			Task: worklog.IDNameField{
				ID:   "CPT-2014",
				Name: "CPT-2014",
			},
			Summary:            "I helped him to get back on track",
			Notes:              "I helped him to get back on track",
			Start:              start,
			BillableDuration:   0,
			UnbillableDuration: time.Second * 1800,
		},
		{
			Client: worklog.IDNameField{
				ID:   "My Awesome Company",
				Name: "My Awesome Company",
			},
			Project: worklog.IDNameField{
				ID:   strconv.Itoa(456),
				Name: "MARVEL",
			},
			Task: worklog.IDNameField{
				ID:   "CPT-MISC",
				Name: "CPT-MISC",
			},
			Summary:            "I helped him to get back on track",
			Notes:              "I helped him to get back on track",
			Start:              start,
			BillableDuration:   0,
			UnbillableDuration: time.Second * 1800,
		},
	}

	mockServer := newMockServer(t, &mockServerOpts{
		Path: toggl.PathWorklog,
		QueryParams: url.Values{
			"page":         {"1"},
			"per_page":     {"50"},
			"since":        {utils.DateFormatISO8601.Format(start)},
			"until":        {utils.DateFormatISO8601.Format(end)},
			"user_id":      {"987654321"},
			"workspace_id": {"123456789"},
			"user_agent":   {"github.com/gabor-boros/minutes"},
		},
		Method:     http.MethodGet,
		StatusCode: http.StatusOK,
		Username:   clientUsername,
		Password:   clientPassword,
		ResponseData: &toggl.FetchResponse{
			TotalCount: 2,
			PerPage:    50,
			Data: []toggl.FetchEntry{
				{
					Client:      "My Awesome Company",
					Description: "I met with The Winter Soldier",
					Duration:    3600000,
					IsBillable:  true,
					Project:     "MARVEL",
					ProjectID:   456,
					Start:       start,
					End:         start.Add(3600000),
					Tags: []string{
						"CPT-2014",
					},
				},
				{
					Client:      "My Awesome Company",
					Description: "I helped him to get back on track",
					Duration:    3600000,
					IsBillable:  false,
					Project:     "MARVEL",
					ProjectID:   456,
					Start:       start,
					End:         start.Add(3600000),
					Tags: []string{
						"CPT-2014",
						"CPT-MISC",
						"IGNORED",
					},
				},
			},
		},
	})
	defer mockServer.Close()

	togglClient, err := toggl.NewFetcher(&toggl.ClientOpts{
		BaseClientOpts: client.BaseClientOpts{
			Timeout: client.DefaultRequestTimeout,
		},
		BasicAuth: client.BasicAuth{
			Username: clientUsername,
			Password: clientPassword,
		},
		BaseURL:   mockServer.URL,
		Workspace: 123456789,
	})
	require.Nil(t, err)

	entries, err := togglClient.FetchEntries(context.Background(), &client.FetchOpts{
		User:             "987654321",
		Start:            start,
		End:              end,
		TagsAsTasksRegex: regexp.MustCompile(`^CPT-\w+$`),
	})

	require.Nil(t, err, "cannot fetch entries")
	require.ElementsMatch(t, expectedEntries, entries, "fetched entries are not matching")
}
