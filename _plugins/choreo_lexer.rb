# _plugins/choreo_lexer.rb

require 'rouge'

class Choreo < Rouge::RegexLexer
  title "Choreo"
  desc "A DSL for behavior-driven testing (github.com/cladam/choreo)"
  tag 'choreo'
  filenames '*.chor'
  mimetypes 'text/x-choreo'

  # List of all known keywords and built-in functions
  KEYWORDS = %w(
    feature actors settings background scenario after test given when then var
  ).freeze

  BUILTINS = %w(
    Web Terminal FileSystem true false
  ).freeze

  COMMANDS = %w(
    # General
    wait
    # Web Actor
    set_header set_cookie http_get http_post clear_header clear_cookie
    # Terminal Actor
    run type wait_for_text
    # FileSystem Actor
    create_file delete_file append_to_file
  ).freeze

  ASSERTIONS = %w(
    # Web Actor
    response_status_is response_time_is_below response_header_is response_body_is response_body_contains
    # Terminal Actor
    expect_exit_code stdout_contains stderr_contains stdout_not_contains stderr_not_contains
    # FileSystem Actor
    file_exists file_not_exists file_contains file_not_contains
  ).freeze

  state :root do
    # Comments
    rule %r/#.*$/, Comment::Single

    # Main structure keywords
    rule %r/\b(#{KEYWORDS.join('|')})\b/, Keyword::Declaration

    # Built-in actors and boolean values
    rule %r/\b(#{BUILTINS.join('|')})\b/, Name::Builtin

    # Actor commands
    rule %r/\b(#{COMMANDS.join('|')})\b/, Name::Function

    # Assertion functions
    rule %r/\b(#{ASSERTIONS.join('|')})\b/, Name::Decorator

    # Variable usage within strings or on their own
    rule %r/\$\{.*?}/, Name::Variable

    # Strings in double quotes
    rule %r/"/, Str::Double, :string

    # Numbers and time values (e.g., 5, 2s, 200)
    rule %r/\b\d+s?\b/, Num

    # Whitespace
    rule %r/\s+/, Text::Whitespace
  end

  # This state handles text inside double quotes, including variable interpolation
  state :string do
    rule %r/"/, Str::Double, :pop!
    rule %r/\$\{.*?}/, Name::Variable # Interpolation within strings
    rule %r/[^"]+/, Str::Double
  end
end
