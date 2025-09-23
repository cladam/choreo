require 'rouge'

module Rouge
  module Lexers
    class Choreo < Rouge::RegexLexer
      title "choreo"
      desc "A DSL for behavior-driven testing (github.com/cladam/choreo)"
      tag 'choreo'
      filenames '*.chor'
      mimetypes 'text/x-choreo'

      # Keywords for blocks and declarations
      KEYWORD_DECLARATION = %w(
        feature actors settings background scenario after test var
      ).freeze

      # Keywords for test steps
      KEYWORD_STEP = %w(
        given when then
      ).freeze

      # Built-in actors and literal values
      BUILTIN_LITERAL = %w(
        Web Terminal FileSystem true false
      ).freeze

      # All commands AND assertions, styled consistently as functions
      COMMANDS_AND_ASSERTIONS = %w(
        wait set_header set_cookie http_get http_post http_put http_patch clear_header clear_cookie
        run type wait_for_text create_file delete_file append_to_file
        response_status_is response_time_is_below response_header_is response_body_is
        response_body_contains expect_exit_code stdout_contains stderr_contains
        stdout_not_contains stderr_not_contains file_exists file_not_exists
        file_contains file_not_contains is_success response_status response_time
        is_below timeout_seconds stop_on_failure shell_path report_path expected_failures
      ).freeze

      state :root do
        # Comments
        rule %r/#.*$/, Comment::Single

        # Whitespace
        rule %r/\s+/, Text::Whitespace

        # Use the arrays defined above to find and tokenise keywords
        rule %r/\b(#{KEYWORD_DECLARATION.join('|')})\b/, Keyword::Declaration
        rule %r/\b(#{KEYWORD_STEP.join('|')})\b/, Keyword
        rule %r/\b(#{BUILTIN_LITERAL.join('|')})\b/, Name::Builtin
        rule %r/\b(#{COMMANDS_AND_ASSERTIONS.join('|')})\b/, Name::Function
        
        # Keywords - MUST come before the generic text rule
        #rule %r/\b(feature|actors|settings|background|scenario|after|test|var)\b/, Keyword::Declaration
        #rule %r/\b(given|when|then)\b/, Keyword

        # Built-ins and commands
        #rule %r/\b(Web|Terminal|FileSystem|true|false)\b/, Name::Builtin
        #rule %r/\b(wait|set_header|set_cookie|http_get|http_post|clear_header|clear_cookie|run|type|wait_for_text|create_file|delete_file|append_to_file|response_status_is|response_time_is_below|response_header_is|response_body_is|response_body_contains|expect_exit_code|stdout_contains|stderr_contains|stdout_not_contains|stderr_not_contains|file_exists|file_not_exists|file_contains|file_not_contains|is_success|timeout_seconds|stop_on_filure|shell_path|report_path|expected_failures)\b/, Name::Function

        # Variable usage
        rule %r/\$\{.*?}/, Name::Variable

        # Strings in double quotes
        rule %r/"/, Str::Double, :string

        # Numbers and time values
        rule %r/\b\d+s?\b/, Num

        # Punctuation and operators
        rule %r/[{}=:]|>=/, Punctuation

        # Any other text
        rule %r/[a-zA-Z_][a-zA-Z0-9_]*/, Text
      end

      state :string do
        rule %r/"/, Str::Double, :pop!
        rule %r/\$\{.*?}/, Name::Variable
        rule %r/[^"]+/, Str::Double
      end
    end
  end
end
