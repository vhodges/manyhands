#!/usr/bin/env bash

set -euo pipefail

script_path=${BASH_SOURCE[0]}
case $script_path in
    */*) script_directory=${script_path%/*} ;;
    *) script_directory=. ;;
esac
project_root=$(cd -- "$script_directory/.." && pwd -P)

fail() {
    printf '%s\n' "$*" >&2
    exit 1
}

assert_contains() {
    local output=$1
    local expected=$2
    local description=$3

    case $output in
        *"$expected"*) ;;
        *)
            printf 'Expected %s to contain %s. Output:\n%s\n' \
                "$description" "$expected" "$output" >&2
            exit 1
            ;;
    esac
}

assert_line() {
    local output=$1
    local expected=$2
    local description=$3
    local line

    while IFS= read -r line || [[ -n $line ]]; do
        if [[ $line == "$expected" ]]; then
            return
        fi
    done <<<"$output"

    printf 'Expected %s to contain exact line %s. Output:\n%s\n' \
        "$description" "$expected" "$output" >&2
    exit 1
}

assert_not_contains() {
    local output=$1
    local unexpected=$2
    local description=$3

    case $output in
        *"$unexpected"*)
            printf 'Expected %s not to contain %s. Output:\n%s\n' \
                "$description" "$unexpected" "$output" >&2
            exit 1
            ;;
    esac
}

assert_equal() {
    local actual=$1
    local expected=$2
    local description=$3

    if [[ $actual != "$expected" ]]; then
        printf 'Expected %s to equal:\n%s\nActual output:\n%s\n' \
            "$description" "$expected" "$actual" >&2
        exit 1
    fi
}

position_of() {
    local output=$1
    local expected=$2
    local prefix

    assert_contains "$output" "$expected" 'ticket details'
    prefix=${output%%"$expected"*}
    printf '%s\n' "${#prefix}"
}

assert_ticket_rows() {
    local output=$1
    local expected_rows rows= output_line

    while IFS= read -r output_line || [[ -n $output_line ]]; do
        if [[ $output_line =~ ^[0-7][0-9A-HJKMNP-TV-Z]{25}$'\t' ]]; then
            rows=${rows:+"$rows"$'\n'}$output_line
        fi
    done <<<"$output"
    expected_rows=$(printf '%s\n' \
        "$nested_override_ticket_row" \
        "$quoted_whitespace_ticket_row" \
        "$escaped_scalar_ticket_row" \
        "$single_quoted_whitespace_ticket_row" \
        "$list_duplicate_ticket_row" \
        "$single_terminal_escape_ticket_row" \
        "$single_quoted_ticket_row" \
        "$inline_comment_ticket_row" \
        "$quoted_scalar_ticket_row" \
        "$primary_ticket_row" \
        "$worktree_ticket_row" \
        "$control_escape_ticket_row" \
        "$unicode_separator_ticket_row" \
        "$c1_control_ticket_row" \
        "$plain_comment_colon_ticket_row")
    assert_equal "$rows" "$expected_rows" 'complete ordered ticket list'
}

assert_malformed_ticket_rejections() {
    local output=$1

    assert_contains "$output" \
        "$collection_title_ticket_id/ticket.md: required field title is not a scalar" \
        'collection-valued ticket diagnostic'
    assert_contains "$output" \
        "$mapping_title_ticket_id/ticket.md: required field title is not a scalar" \
        'mapping-valued ticket diagnostic'
    assert_contains "$output" \
        "$comment_title_ticket_id/ticket.md:" \
        'comment-only ticket diagnostic'
    assert_contains "$output" \
        "$block_scalar_title_ticket_id/ticket.md: required field title is not a scalar" \
        'block-scalar ticket diagnostic'
    assert_contains "$output" \
        "$escaped_terminal_quote_ticket_id/ticket.md: required field title is not a scalar" \
        'escaped-terminal-quote ticket diagnostic'
    assert_contains "$output" \
        "$unterminated_single_quote_ticket_id/ticket.md: required field title is not a scalar" \
        'unterminated-single-quote ticket diagnostic'
    assert_contains "$output" \
        "$nested_required_ticket_id/ticket.md: missing a required nonempty scalar field" \
        'nested required-field ticket diagnostic'
    assert_contains "$output" \
        "$duplicate_metadata_ticket_id/ticket.md: duplicate field title" \
        'duplicate ticket metadata diagnostic'
    assert_contains "$output" \
        "$mismatched_directory_ticket_id/ticket.md: ticket ID does not match its directory" \
        'ticket directory-ID mismatch diagnostic'
    assert_contains "$output" \
        "$noncanonical_id_ticket_id/ticket.md: ticket ID is not a canonical ULID" \
        'noncanonical ticket ID diagnostic'
    assert_contains "$output" \
        "not-a-ticket-id/ticket.md: ticket directory name is not a canonical ULID" \
        'noncanonical ticket directory diagnostic'
    assert_contains "$output" \
        "$invalid_escape_ticket_id/ticket.md: required field title is not a scalar" \
        'invalid escape ticket diagnostic'
    assert_contains "$output" \
        "$invalid_trailing_ticket_id/ticket.md: required field title is not a scalar" \
        'invalid trailing text ticket diagnostic'
    assert_contains "$output" \
        "$plain_mapping_ticket_id/ticket.md: required field title is not a scalar" \
        'plain mapping ticket diagnostic'
    assert_contains "$output" \
        "$list_duplicate_ticket_id/ticket.md: duplicate canonical ticket ID" \
        'duplicate ticket collision diagnostic'
    case $output in
        *"${collection_title_ticket_id}"$'\t'*)
            fail 'collection-valued ticket was listed'
            ;;
        *"${mapping_title_ticket_id}"$'\t'*)
            fail 'mapping-valued ticket was listed'
            ;;
        *"${comment_title_ticket_id}"$'\t'*)
            fail 'comment-only ticket was listed'
            ;;
        *"${block_scalar_title_ticket_id}"$'\t'*)
            fail 'block-scalar ticket was listed'
            ;;
        *"${escaped_terminal_quote_ticket_id}"$'\t'*)
            fail 'escaped-terminal-quote ticket was listed'
            ;;
        *"${unterminated_single_quote_ticket_id}"$'\t'*)
            fail 'unterminated-single-quote ticket was listed'
            ;;
    esac
}

assert_ticket_details() {
    local output=$1
    local expected_output
    local oldest_comment_position
    local middle_comment_position
    local same_time_comment_position
    local fractional_earlier_comment_position
    local fractional_later_comment_position
    local equal_instant_lower_comment_position
    local equal_instant_higher_comment_position
    local newest_comment_position

    assert_line "$output" "ID: $worktree_ticket_id" 'ticket details'
    assert_line "$output" 'Status: done' 'ticket details'
    assert_line "$output" 'Type: bug' 'ticket details'
    assert_line "$output" 'Title: Worktree ticket # title' 'ticket details'
    assert_contains "$output" 'Worktree ticket details.' 'ticket details'
    assert_contains "$output" $'Comments:\n2026-10-01T09:00:00Z '"$oldest_comment_id" \
        'ticket details'
    expected_output=$(printf '%s\n' \
        "ID: $worktree_ticket_id" \
        'Status: done' \
        'Type: bug' \
        'Title: Worktree ticket # title' \
        '' \
        '' \
        '# Worktree ticket body' \
        '' \
        'Worktree ticket details.' \
        'Comments:' \
        "2026-10-01T09:00:00Z $oldest_comment_id" \
        '' \
        'Oldest worktree comment body.' \
        '' \
        "2026-10-01T09:30:00Z $middle_comment_id" \
        '' \
        'Middle worktree comment body.' \
        '' \
        "2026-10-01T09:30:00Z $same_time_comment_id" \
        '' \
        'Same-time worktree comment body.' \
        '' \
        "2026-10-01T09:45:00.09Z $fractional_earlier_comment_id" \
        '' \
        'Fractional earlier worktree comment body.' \
        '' \
        "2026-10-01T09:45:00.1Z $fractional_later_comment_id" \
        '' \
        'Fractional later worktree comment body.' \
        '' \
        "2026-10-01T09:50:00.1Z $equal_instant_lower_comment_id" \
        '' \
        'Equal-instant lower-ID worktree comment body.' \
        '' \
        "2026-10-01T09:50:00.10Z $equal_instant_higher_comment_id" \
        '' \
        'Equal-instant higher-ID worktree comment body.' \
        '' \
        "2026-10-01T10:00:00Z $newest_comment_id" \
        '' \
        'Newest worktree comment body.')
    assert_equal "$output" "$expected_output" 'ticket details'
    oldest_comment_position=$(position_of "$output" 'Oldest worktree comment body.')
    middle_comment_position=$(position_of "$output" 'Middle worktree comment body.')
    same_time_comment_position=$(position_of "$output" 'Same-time worktree comment body.')
    fractional_earlier_comment_position=$(position_of "$output" \
        'Fractional earlier worktree comment body.')
    fractional_later_comment_position=$(position_of "$output" \
        'Fractional later worktree comment body.')
    equal_instant_lower_comment_position=$(position_of "$output" \
        'Equal-instant lower-ID worktree comment body.')
    equal_instant_higher_comment_position=$(position_of "$output" \
        'Equal-instant higher-ID worktree comment body.')
    newest_comment_position=$(position_of "$output" 'Newest worktree comment body.')
    if (( oldest_comment_position >= middle_comment_position \
        || middle_comment_position >= same_time_comment_position \
        || same_time_comment_position >= fractional_earlier_comment_position \
        || fractional_earlier_comment_position >= fractional_later_comment_position \
        || fractional_later_comment_position >= equal_instant_lower_comment_position \
        || equal_instant_lower_comment_position >= equal_instant_higher_comment_position \
        || equal_instant_higher_comment_position >= newest_comment_position )); then
        fail 'worktree comments were not displayed in created_at and ID order'
    fi
}

assert_no_comments() {
    local output=$1

    assert_line "$output" "ID: $primary_ticket_id" 'ticket details without comments'
    assert_contains "$output" 'Primary ticket details.' 'ticket details without comments'
    assert_line "$output" 'Comments: none' 'ticket details without comments'
}

assert_invalid_ticket_id() {
    local description=$1
    shift
    local output

    if output=$(cd -- "$nested_worktree_directory" && \
        "$project_root/scripts/show-ticket" "$@" 2>&1); then
        fail "show-ticket accepted $description"
    fi
    assert_contains "$output" 'canonical uppercase ticket ULID' "$description diagnostic"
}

temporary_root=$(mktemp -d)
trap 'rm -rf "$temporary_root"' EXIT

repository=$temporary_root/repository
external_git_directory=$temporary_root/external-git
empty_repository=$temporary_root/empty-repository
primary_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FAW
worktree_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FAX
collection_title_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FAS
mapping_title_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FAT
comment_title_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FAU
block_scalar_title_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FAR
inline_comment_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FAP
quoted_scalar_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FAQ
escaped_terminal_quote_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FAM
single_quoted_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FAN
unterminated_single_quote_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FAK
single_terminal_escape_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FAJ
middle_comment_id=01ARZ3NDEKTSV4RRFFQ69G5FAV
newest_comment_id=01ARZ3NDEKTSV4RRFFQ69G5FAY
oldest_comment_id=01ARZ3NDEKTSV4RRFFQ69G5FAZ
same_time_comment_id=01ARZ3NDEKTSV4RRFFQ69G5FB2
fractional_earlier_comment_id=01ARZ3NDEKTSV4RRFFQ69G5FB3
fractional_later_comment_id=01ARZ3NDEKTSV4RRFFQ69G5FB4
equal_instant_lower_comment_id=01ARZ3NDEKTSV4RRFFQ69G5FB5
equal_instant_higher_comment_id=01ARZ3NDEKTSV4RRFFQ69G5FB6
malformed_duplicate_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FB7
malformed_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FB8
invalid_seconds_comment_id=01ARZ3NDEKTSV4RRFFQ69G5FB9
invalid_comment_id=01ARZ3NDEKTSV4RRFFQ69G5FB1
invalid_nested_comment_id=01ARZ3NDEKTSV4RRFFQ69G5FBA
duplicate_comment_metadata_id=01ARZ3NDEKTSV4RRFFQ69G5FBB
unknown_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FB0
nested_override_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FAA
quoted_whitespace_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FAB
escaped_scalar_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FAC
single_quoted_whitespace_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FAD
list_duplicate_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FAE
nested_required_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FAF
duplicate_metadata_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FAG
mismatched_directory_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FAH
noncanonical_id_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FAV
invalid_escape_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FAY
invalid_trailing_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FAZ
control_escape_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FBC
unicode_separator_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FBD
c1_control_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FBE
plain_mapping_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FBF
plain_comment_colon_ticket_id=01ARZ3NDEKTSV4RRFFQ69G5FBG
primary_ticket_row="${primary_ticket_id}"$'\topen\ttask\tPrimary ticket title'
worktree_ticket_row="${worktree_ticket_id}"$'\tdone\tbug\tWorktree ticket # title'
inline_comment_ticket_row="${inline_comment_ticket_id}"$'\topen\ttask\tPlain#title'
quoted_scalar_ticket_row="${quoted_scalar_ticket_id}"$'\topen "state"\ttask "kind"\tQuoted "ticket" # title'
single_quoted_ticket_row="${single_quoted_ticket_id}"$'\topen\ttask\tOwner\'s ticket'
nested_override_ticket_row="${nested_override_ticket_id}"$'\topen\ttask\tTop-level title'
quoted_whitespace_ticket_row="${quoted_whitespace_ticket_id}"$'\topen\ttask\t  Preserved quoted title whitespace  '
escaped_scalar_ticket_row="${escaped_scalar_ticket_id}"$'\topen\ttask\tEscaped " slash \\\\ bang ! unicode A'
single_quoted_whitespace_ticket_row="${single_quoted_whitespace_ticket_id}"$'\topen\ttask\t  Owner\'s quoted title  '
list_duplicate_ticket_row="${list_duplicate_ticket_id}"$'\topen\ttask\tPrimary duplicate ticket'
control_escape_ticket_row="${control_escape_ticket_id}"$'\topen\\nstate\ttask\\tkind\tControl\\nTitle\\tPath\\\\Carriage\\rReturn'
unicode_separator_ticket_row="${unicode_separator_ticket_id}"$'\topen\\u0085state\ttask\\u2028kind\tTitle\\u2029finish'
c1_control_ticket_row="${c1_control_ticket_id}"$'\topen\ttask\tC1\\u009Bfinish'
plain_comment_colon_ticket_row="${plain_comment_colon_ticket_id}"$'\topen\ttask\tValid plain title'
lowercase_worktree_ticket_id=$(printf '%s' "$worktree_ticket_id" | tr '[:upper:]' '[:lower:]')
single_terminal_escape_ticket_row="${single_terminal_escape_ticket_id}"$'\topen\ttask\t'"Owner'"

git init -q --initial-branch=main --separate-git-dir "$external_git_directory" "$repository"
git -C "$repository" config user.name 'Interim Ticket Test'
git -C "$repository" config user.email 'interim-ticket-test@example.invalid'
mkdir -p "$repository/.manyhands"
printf '%s\n' \
    'format_version = 1' \
    'primary_branch = "main"' \
    >"$repository/.manyhands/config.toml"
git -C "$repository" add .manyhands/config.toml
git -C "$repository" commit -q -m 'Add Manyhands configuration'

worktree=$repository/.manyhands/worktrees/$worktree_ticket_id
git -C "$repository" worktree add -q -b "manyhands/ticket/$worktree_ticket_id" "$worktree" HEAD

mkdir -p "$repository/.manyhands/tickets/$primary_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: $primary_ticket_id" \
    'title: Primary ticket title' \
    'type: task' \
    'status: open' \
    '---' \
    '' \
    '# Primary ticket body' \
    '' \
    'Primary ticket details.' \
    >"$repository/.manyhands/tickets/$primary_ticket_id/ticket.md"

mkdir -p "$repository/.manyhands/tickets/$collection_title_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: $collection_title_ticket_id" \
    'title: [not, a, scalar]' \
    'type: task' \
    'status: open' \
    '---' \
    >"$repository/.manyhands/tickets/$collection_title_ticket_id/ticket.md"

mkdir -p "$repository/.manyhands/tickets/$mapping_title_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: $mapping_title_ticket_id" \
    'title: {key: value}' \
    'type: task' \
    'status: open' \
    '---' \
    >"$repository/.manyhands/tickets/$mapping_title_ticket_id/ticket.md"

mkdir -p "$repository/.manyhands/tickets/$comment_title_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: $comment_title_ticket_id" \
    'title: # comment-only value is YAML null' \
    'type: task' \
    'status: open' \
    '---' \
    >"$repository/.manyhands/tickets/$comment_title_ticket_id/ticket.md"

mkdir -p "$repository/.manyhands/tickets/$block_scalar_title_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: $block_scalar_title_ticket_id" \
    'title: |-' \
    '  Block scalar title value.' \
    'type: task' \
    'status: open' \
    '---' \
    >"$repository/.manyhands/tickets/$block_scalar_title_ticket_id/ticket.md"

mkdir -p "$repository/.manyhands/tickets/$inline_comment_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: $inline_comment_ticket_id" \
    'title: Plain#title # trailing comment' \
    'type: task # trailing comment' \
    'status: open # trailing comment' \
    '---' \
    >"$repository/.manyhands/tickets/$inline_comment_ticket_id/ticket.md"

mkdir -p "$repository/.manyhands/tickets/$quoted_scalar_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: \"$quoted_scalar_ticket_id\"" \
    'title: "Quoted \"ticket\" # title"' \
    'type: "task \"kind\""' \
    'status: "open \"state\""' \
    '---' \
    >"$repository/.manyhands/tickets/$quoted_scalar_ticket_id/ticket.md"

mkdir -p "$repository/.manyhands/tickets/$escaped_terminal_quote_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: $escaped_terminal_quote_ticket_id" \
    'title: "unterminated \"' \
    'type: task' \
    'status: open' \
    '---' \
    >"$repository/.manyhands/tickets/$escaped_terminal_quote_ticket_id/ticket.md"

mkdir -p "$repository/.manyhands/tickets/$single_quoted_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: $single_quoted_ticket_id" \
    "title: 'Owner''s ticket'" \
    'type: task' \
    'status: open' \
    '---' \
    >"$repository/.manyhands/tickets/$single_quoted_ticket_id/ticket.md"

mkdir -p "$repository/.manyhands/tickets/$unterminated_single_quote_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: $unterminated_single_quote_ticket_id" \
    "title: 'unterminated ''" \
    'type: task' \
    'status: open' \
    '---' \
    >"$repository/.manyhands/tickets/$unterminated_single_quote_ticket_id/ticket.md"

mkdir -p "$repository/.manyhands/tickets/$single_terminal_escape_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: $single_terminal_escape_ticket_id" \
    "title: 'Owner'''" \
    'type: task' \
    'status: open' \
    '---' \
    >"$repository/.manyhands/tickets/$single_terminal_escape_ticket_id/ticket.md"

mkdir -p "$repository/.manyhands/tickets/$nested_override_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: $nested_override_ticket_id" \
    'title: Top-level title' \
    'metadata:' \
    '  title: Nested title must not overwrite' \
    'type: task' \
    'status: open' \
    '---' \
    >"$repository/.manyhands/tickets/$nested_override_ticket_id/ticket.md"
mkdir -p "$repository/.manyhands/tickets/$quoted_whitespace_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: \"$quoted_whitespace_ticket_id\" # ID comment" \
    'title: "  Preserved quoted title whitespace  " # title comment' \
    'type: "task" # type comment' \
    'status: "open" # status comment' \
    '---' \
    >"$repository/.manyhands/tickets/$quoted_whitespace_ticket_id/ticket.md"
mkdir -p "$repository/.manyhands/tickets/$escaped_scalar_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: $escaped_scalar_ticket_id" \
    'title: "Escaped \" slash \\ bang \x21 unicode \u0041"' \
    'type: task' \
    'status: open' \
    '---' \
    >"$repository/.manyhands/tickets/$escaped_scalar_ticket_id/ticket.md"
mkdir -p "$repository/.manyhands/tickets/$single_quoted_whitespace_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: $single_quoted_whitespace_ticket_id" \
    "title: '  Owner''s quoted title  ' # title comment" \
    'type: task' \
    'status: open' \
    '---' \
    >"$repository/.manyhands/tickets/$single_quoted_whitespace_ticket_id/ticket.md"
mkdir -p "$repository/.manyhands/tickets/$list_duplicate_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: $list_duplicate_ticket_id" \
    'title: Primary duplicate ticket' \
    'type: task' \
    'status: open' \
    '---' \
    >"$repository/.manyhands/tickets/$list_duplicate_ticket_id/ticket.md"
mkdir -p "$worktree/.manyhands/tickets/$list_duplicate_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: $list_duplicate_ticket_id" \
    'title: Worktree duplicate ticket' \
    'type: task' \
    'status: open' \
    '---' \
    >"$worktree/.manyhands/tickets/$list_duplicate_ticket_id/ticket.md"
mkdir -p "$repository/.manyhands/tickets/$nested_required_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: $nested_required_ticket_id" \
    'metadata:' \
    '  title: Nested title must not fulfill the required field' \
    'type: task' \
    'status: open' \
    '---' \
    >"$repository/.manyhands/tickets/$nested_required_ticket_id/ticket.md"
mkdir -p "$repository/.manyhands/tickets/$duplicate_metadata_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: $duplicate_metadata_ticket_id" \
    'title: First title' \
    'title: Duplicate title' \
    'type: task' \
    'status: open' \
    '---' \
    >"$repository/.manyhands/tickets/$duplicate_metadata_ticket_id/ticket.md"
mkdir -p "$repository/.manyhands/tickets/$mismatched_directory_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: $nested_override_ticket_id" \
    'title: Mismatched directory ticket' \
    'type: task' \
    'status: open' \
    '---' \
    >"$repository/.manyhands/tickets/$mismatched_directory_ticket_id/ticket.md"
mkdir -p "$repository/.manyhands/tickets/$noncanonical_id_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    'id: not-a-ticket-id' \
    'title: Noncanonical ID ticket' \
    'type: task' \
    'status: open' \
    '---' \
    >"$repository/.manyhands/tickets/$noncanonical_id_ticket_id/ticket.md"
mkdir -p "$repository/.manyhands/tickets/not-a-ticket-id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: $noncanonical_id_ticket_id" \
    'title: Noncanonical directory ticket' \
    'type: task' \
    'status: open' \
    '---' \
    >"$repository/.manyhands/tickets/not-a-ticket-id/ticket.md"
mkdir -p "$repository/.manyhands/tickets/$invalid_escape_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: $invalid_escape_ticket_id" \
    'title: "Invalid \q escape"' \
    'type: task' \
    'status: open' \
    '---' \
    >"$repository/.manyhands/tickets/$invalid_escape_ticket_id/ticket.md"
mkdir -p "$repository/.manyhands/tickets/$invalid_trailing_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: $invalid_trailing_ticket_id" \
    'title: "Unexpected trailing value" not-a-comment' \
    'type: task' \
    'status: open' \
    '---' \
    >"$repository/.manyhands/tickets/$invalid_trailing_ticket_id/ticket.md"

mkdir -p "$repository/.manyhands/tickets/$control_escape_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: $control_escape_ticket_id" \
    'title: "Control\nTitle\tPath\\Carriage\rReturn"' \
    'type: "task\tkind"' \
    'status: "open\nstate"' \
    '---' \
    >"$repository/.manyhands/tickets/$control_escape_ticket_id/ticket.md"

mkdir -p "$repository/.manyhands/tickets/$unicode_separator_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: $unicode_separator_ticket_id" \
    'title: "Title\Pfinish"' \
    'type: "task\Lkind"' \
    'status: "open\Nstate"' \
    '---' \
    >"$repository/.manyhands/tickets/$unicode_separator_ticket_id/ticket.md"

mkdir -p "$repository/.manyhands/tickets/$c1_control_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: $c1_control_ticket_id" \
    'title: "C1\u009Bfinish"' \
    'type: task' \
    'status: open' \
    '---' \
    >"$repository/.manyhands/tickets/$c1_control_ticket_id/ticket.md"

mkdir -p "$repository/.manyhands/tickets/$plain_mapping_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: $plain_mapping_ticket_id" \
    'title: valid: nested' \
    'type: task' \
    'status: open' \
    '---' \
    >"$repository/.manyhands/tickets/$plain_mapping_ticket_id/ticket.md"

mkdir -p "$repository/.manyhands/tickets/$plain_comment_colon_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: $plain_comment_colon_ticket_id" \
    'title: Valid plain title # comment: nested mapping syntax' \
    'type: task' \
    'status: open' \
    '---' \
    >"$repository/.manyhands/tickets/$plain_comment_colon_ticket_id/ticket.md"

mkdir -p "$worktree/.manyhands/tickets/$worktree_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: \"$worktree_ticket_id\"" \
    'title: "Worktree ticket # title"' \
    'type: "bug"' \
    'status: "done"' \
    '---' \
    '' \
    '# Worktree ticket body' \
    '' \
    'Worktree ticket details.' \
    >"$worktree/.manyhands/tickets/$worktree_ticket_id/ticket.md"

mkdir -p "$worktree/.manyhands/comments/$worktree_ticket_id"
# Filename order is middle, newest, oldest; only created_at produces the expected order.
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: comment' \
    "id: $middle_comment_id" \
    "item_id: $worktree_ticket_id" \
    'created_at: 2026-10-01T09:30:00Z' \
    '---' \
    '' \
    'Middle worktree comment body.' \
    >"$worktree/.manyhands/comments/$worktree_ticket_id/$middle_comment_id.md"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: comment' \
    "id: $newest_comment_id" \
    "item_id: $worktree_ticket_id" \
    'created_at: 2026-10-01T10:00:00Z' \
    '---' \
    '' \
    'Newest worktree comment body.' \
    >"$worktree/.manyhands/comments/$worktree_ticket_id/$newest_comment_id.md"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: comment' \
    "id: $oldest_comment_id" \
    "item_id: $worktree_ticket_id" \
    'created_at: 2026-10-01T09:00:00Z' \
    '---' \
    '' \
    'Oldest worktree comment body.' \
    >"$worktree/.manyhands/comments/$worktree_ticket_id/$oldest_comment_id.md"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: comment' \
    "id: $same_time_comment_id" \
    "item_id: $worktree_ticket_id" \
    'created_at: 2026-10-01T09:30:00Z' \
    '---' \
    '' \
    'Same-time worktree comment body.' \
    >"$worktree/.manyhands/comments/$worktree_ticket_id/$same_time_comment_id.md"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: comment' \
    "id: $fractional_earlier_comment_id" \
    "item_id: $worktree_ticket_id" \
    'created_at: 2026-10-01T09:45:00.09Z' \
    '---' \
    '' \
    'Fractional earlier worktree comment body.' \
    >"$worktree/.manyhands/comments/$worktree_ticket_id/$fractional_earlier_comment_id.md"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: comment' \
    "id: $fractional_later_comment_id" \
    "item_id: $worktree_ticket_id" \
    'created_at: 2026-10-01T09:45:00.1Z' \
    '---' \
    '' \
    'Fractional later worktree comment body.' \
    >"$worktree/.manyhands/comments/$worktree_ticket_id/$fractional_later_comment_id.md"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: comment' \
    "id: $equal_instant_lower_comment_id" \
    "item_id: $worktree_ticket_id" \
    'created_at: 2026-10-01T09:50:00.1Z' \
    '---' \
    '' \
    'Equal-instant lower-ID worktree comment body.' \
    >"$worktree/.manyhands/comments/$worktree_ticket_id/$equal_instant_lower_comment_id.md"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: comment' \
    "id: $equal_instant_higher_comment_id" \
    "item_id: $worktree_ticket_id" \
    'created_at: 2026-10-01T09:50:00.10Z' \
    '---' \
    '' \
    'Equal-instant higher-ID worktree comment body.' \
    >"$worktree/.manyhands/comments/$worktree_ticket_id/$equal_instant_higher_comment_id.md"

nested_primary_directory=$repository/nested/directory
nested_worktree_directory=$worktree/nested/directory
mkdir -p "$nested_primary_directory" "$nested_worktree_directory"

for nested_directory in "$nested_primary_directory" "$nested_worktree_directory"; do
    if ! list_output=$(cd -- "$nested_directory" && "$project_root/scripts/list-tickets" 2>&1); then
        fail "list-tickets failed unexpectedly from $nested_directory: $list_output"
    fi
    assert_ticket_rows "$list_output"
    assert_malformed_ticket_rejections "$list_output"
    assert_not_contains "$list_output" $'\nstate\ttask\tkind\tControl' \
        'forged control-escape TSV row'
    assert_not_contains "$list_output" $'\302\205' 'raw Unicode NEL TSV field'
    assert_not_contains "$list_output" $'\342\200\250' 'raw Unicode line-separator TSV field'
    assert_not_contains "$list_output" $'\342\200\251' 'raw Unicode paragraph-separator TSV field'
    assert_not_contains "$list_output" $'\302\233' 'raw Unicode C1 TSV field'
done

if ! show_output=$(cd -- "$nested_worktree_directory" && "$project_root/scripts/show-ticket" "$worktree_ticket_id" 2>&1); then
    fail "show-ticket failed unexpectedly from $nested_worktree_directory: $show_output"
fi
assert_ticket_details "$show_output"

if ! no_comments_output=$(cd -- "$nested_primary_directory" && \
    "$project_root/scripts/show-ticket" "$primary_ticket_id" 2>&1); then
    fail "show-ticket failed unexpectedly for a ticket without comments: $no_comments_output"
fi
assert_no_comments "$no_comments_output"

if nested_ticket_output=$(cd -- "$nested_primary_directory" && \
    "$project_root/scripts/show-ticket" "$nested_required_ticket_id" 2>&1); then
    fail 'show-ticket accepted a ticket whose required title exists only in a nested map'
fi
assert_contains "$nested_ticket_output" "ticket $nested_required_ticket_id is malformed" \
    'nested ticket diagnostic'

if duplicate_ticket_output=$(cd -- "$nested_primary_directory" && \
    "$project_root/scripts/show-ticket" "$duplicate_metadata_ticket_id" 2>&1); then
    fail 'show-ticket accepted duplicate ticket metadata'
fi
assert_contains "$duplicate_ticket_output" 'duplicate field title' 'duplicate ticket diagnostic'

if ! quoted_ticket_output=$(cd -- "$nested_primary_directory" && \
    "$project_root/scripts/show-ticket" "$quoted_whitespace_ticket_id" 2>&1); then
    fail "show-ticket failed for a valid quoted scalar: $quoted_ticket_output"
fi
assert_line "$quoted_ticket_output" 'Title:   Preserved quoted title whitespace  ' \
    'quoted whitespace title'

if ! escaped_ticket_output=$(cd -- "$nested_primary_directory" && \
    "$project_root/scripts/show-ticket" "$escaped_scalar_ticket_id" 2>&1); then
    fail "show-ticket failed for valid escaped scalars: $escaped_ticket_output"
fi
assert_line "$escaped_ticket_output" 'Title: Escaped " slash \\ bang ! unicode A' \
    'decoded escaped title'

if ! control_escape_ticket_output=$(cd -- "$nested_primary_directory" && \
    "$project_root/scripts/show-ticket" "$control_escape_ticket_id" 2>&1); then
    fail "show-ticket failed for control escapes: $control_escape_ticket_output"
fi
assert_line "$control_escape_ticket_output" 'Status: open\nstate' 'escaped status display'
assert_line "$control_escape_ticket_output" 'Type: task\tkind' 'escaped type display'
assert_line "$control_escape_ticket_output" \
    'Title: Control\nTitle\tPath\\Carriage\rReturn' 'escaped title display'
assert_not_contains "$control_escape_ticket_output" $'\nstate\nType:' \
    'forged control-escape show header'

if ! unicode_separator_ticket_output=$(cd -- "$nested_primary_directory" && \
    "$project_root/scripts/show-ticket" "$unicode_separator_ticket_id" 2>&1); then
    fail "show-ticket failed for Unicode separators: $unicode_separator_ticket_output"
fi
assert_line "$unicode_separator_ticket_output" 'Status: open\u0085state' \
    'escaped Unicode NEL display'
assert_line "$unicode_separator_ticket_output" 'Type: task\u2028kind' \
    'escaped Unicode line-separator display'
assert_line "$unicode_separator_ticket_output" 'Title: Title\u2029finish' \
    'escaped Unicode paragraph-separator display'
assert_not_contains "$unicode_separator_ticket_output" $'\302\205' \
    'raw Unicode NEL display'
assert_not_contains "$unicode_separator_ticket_output" $'\342\200\250' \
    'raw Unicode line-separator display'
assert_not_contains "$unicode_separator_ticket_output" $'\342\200\251' \
    'raw Unicode paragraph-separator display'

if ! c1_control_ticket_output=$(cd -- "$nested_primary_directory" && \
    "$project_root/scripts/show-ticket" "$c1_control_ticket_id" 2>&1); then
    fail "show-ticket failed for C1 controls: $c1_control_ticket_output"
fi
assert_line "$c1_control_ticket_output" 'Title: C1\u009Bfinish' \
    'escaped Unicode C1 control display'
assert_not_contains "$c1_control_ticket_output" $'\302\233' \
    'raw Unicode C1 control display'

mkdir -p "$worktree/.manyhands/comments/$worktree_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: comment' \
    "id: $invalid_comment_id" \
    "item_id: $worktree_ticket_id" \
    'created_at: 2026-10-01T11:00:00+00:00' \
    '---' \
    '' \
    'Rejected comment body.' \
    >"$worktree/.manyhands/comments/$worktree_ticket_id/$invalid_comment_id.md"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: comment' \
    "id: $invalid_seconds_comment_id" \
    "item_id: $worktree_ticket_id" \
    'created_at: 2026-10-01T11:00:60Z' \
    '---' \
    '' \
    'Invalid seconds comment body.' \
    >"$worktree/.manyhands/comments/$worktree_ticket_id/$invalid_seconds_comment_id.md"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: comment' \
    "id: $invalid_nested_comment_id" \
    "item_id: $worktree_ticket_id" \
    'metadata:' \
    '  created_at: 2026-10-01T11:01:00Z' \
    '---' \
    '' \
    'Nested metadata comment body.' \
    >"$worktree/.manyhands/comments/$worktree_ticket_id/$invalid_nested_comment_id.md"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: comment' \
    "id: $duplicate_comment_metadata_id" \
    "item_id: $worktree_ticket_id" \
    'created_at: 2026-10-01T11:02:00Z' \
    'created_at: 2026-10-01T11:03:00Z' \
    '---' \
    '' \
    'Duplicate metadata comment body.' \
    >"$worktree/.manyhands/comments/$worktree_ticket_id/$duplicate_comment_metadata_id.md"
if ! invalid_comment_output=$(cd -- "$nested_worktree_directory" && \
    "$project_root/scripts/show-ticket" "$worktree_ticket_id" 2>&1); then
    fail "show-ticket failed unexpectedly with an invalid comment: $invalid_comment_output"
fi
assert_contains "$invalid_comment_output" 'created_at is not a valid RFC3339 UTC timestamp' \
    'invalid comment diagnostic'
assert_contains "$invalid_comment_output" \
    "$invalid_seconds_comment_id.md: created_at is not a valid RFC3339 UTC timestamp" \
    'invalid seconds diagnostic'
assert_contains "$invalid_comment_output" \
    "$invalid_nested_comment_id.md: missing a required nonempty scalar field" \
    'nested comment diagnostic'
assert_contains "$invalid_comment_output" \
    "$duplicate_comment_metadata_id.md: duplicate field created_at" \
    'duplicate comment metadata diagnostic'
assert_not_contains "$invalid_comment_output" 'Rejected comment body.' 'displayed invalid comment'
assert_not_contains "$invalid_comment_output" 'Invalid seconds comment body.' \
    'displayed invalid-seconds comment'
assert_not_contains "$invalid_comment_output" 'Nested metadata comment body.' \
    'displayed nested metadata comment'
assert_not_contains "$invalid_comment_output" 'Duplicate metadata comment body.' \
    'displayed duplicate metadata comment'

assert_invalid_ticket_id 'a missing ticket ID'
assert_invalid_ticket_id 'an extra ticket ID' "$worktree_ticket_id" "$primary_ticket_id"
assert_invalid_ticket_id 'a lowercase ticket ID' "$lowercase_worktree_ticket_id"
assert_invalid_ticket_id 'a malformed ticket ID' 'not-a-ticket-id'
assert_invalid_ticket_id 'a noncanonical ticket ID' "${worktree_ticket_id/0/8}"

mkdir -p "$repository/.manyhands/tickets/$malformed_duplicate_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: $malformed_duplicate_ticket_id" \
    'title: Valid duplicate candidate' \
    'type: task' \
    'status: open' \
    '---' \
    >"$repository/.manyhands/tickets/$malformed_duplicate_ticket_id/ticket.md"
mkdir -p "$worktree/.manyhands/tickets/$malformed_duplicate_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: $malformed_duplicate_ticket_id" \
    'title: [Malformed duplicate candidate]' \
    'type: task' \
    'status: open' \
    '---' \
    >"$worktree/.manyhands/tickets/$malformed_duplicate_ticket_id/ticket.md"
if malformed_duplicate_output=$(cd -- "$nested_primary_directory" && \
    "$project_root/scripts/show-ticket" "$malformed_duplicate_ticket_id" 2>&1); then
    fail 'show-ticket accepted duplicate candidates with malformed metadata'
fi
assert_contains "$malformed_duplicate_output" 'multiple canonical tickets' \
    'malformed duplicate-ticket diagnostic'

mkdir -p "$repository/.manyhands/tickets/$malformed_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: $malformed_ticket_id" \
    'title: [Malformed ticket]' \
    'type: task' \
    'status: open' \
    '---' \
    >"$repository/.manyhands/tickets/$malformed_ticket_id/ticket.md"
if malformed_ticket_output=$(cd -- "$nested_primary_directory" && \
    "$project_root/scripts/show-ticket" "$malformed_ticket_id" 2>&1); then
    fail 'show-ticket accepted a malformed ticket'
fi
assert_contains "$malformed_ticket_output" \
    "ticket $malformed_ticket_id is malformed" 'malformed-ticket diagnostic'
assert_not_contains "$malformed_ticket_output" 'not found' 'malformed-ticket diagnostic'

mkdir -p "$repository/.manyhands/tickets/$worktree_ticket_id"
printf '%s\n' \
    '---' \
    'manyhands_managed: true' \
    'manyhands_kind: ticket' \
    "id: $worktree_ticket_id" \
    'title: Duplicate worktree ticket' \
    'type: bug' \
    'status: done' \
    '---' \
    >"$repository/.manyhands/tickets/$worktree_ticket_id/ticket.md"
if duplicate_output=$(cd -- "$nested_primary_directory" && \
    "$project_root/scripts/show-ticket" "$worktree_ticket_id" 2>&1); then
    fail 'show-ticket succeeded for duplicate canonical tickets'
fi
assert_contains "$duplicate_output" 'multiple canonical tickets' 'duplicate-ticket diagnostic'

if unknown_output=$(cd -- "$nested_primary_directory" && "$project_root/scripts/show-ticket" "$unknown_ticket_id" 2>&1); then
    fail 'show-ticket succeeded for an unknown ticket'
fi
assert_contains "$unknown_output" "$unknown_ticket_id" 'unknown-ticket diagnostic'
case $unknown_output in
    *not\ found*|*unknown*|*does\ not\ exist*) ;;
    *) fail "unknown-ticket diagnostic was unclear: $unknown_output" ;;
esac

git init -q --initial-branch=main "$empty_repository"
mkdir -p "$empty_repository/.manyhands"
printf '%s\n' \
    'format_version = 1' \
    'primary_branch = "main"' \
    >"$empty_repository/.manyhands/config.toml"
if ! empty_output=$(cd -- "$empty_repository" && "$project_root/scripts/list-tickets" 2>&1); then
    fail "list-tickets failed for an empty configured repository: $empty_output"
fi
ticket_row_pattern='^[0-7][0-9A-HJKMNP-TV-Z]{25}([[:space:]]|$)'
while IFS= read -r output_line || [[ -n $output_line ]]; do
    if [[ $output_line =~ $ticket_row_pattern ]]; then
        fail "empty configured repository listed ticket records: $empty_output"
    fi
done <<<"$empty_output"

printf '%s\n' 'interim ticket script acceptance test passed'
