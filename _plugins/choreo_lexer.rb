# _plugins/choreo_lexer.rb

require 'rouge'

class Choreo < Rouge::RegexLexer
  title "Choreo"
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

  # Actor commands (actions)
  COMMANDS = %w(
    wait set_header set_cookie http_get http_post clear_header clear_cookie
    run type wait_for_text create_file delete_file append_to_file
  ).freeze

  # Assertions (conditions)
  ASSERTIONS = %w(
    response_status_is response_time_is_below response_header_is response_body_is
    response_body_contains expect_exit_code stdout_contains stderr_contains
    stdout_not_contains stderr_not_contains file_exists file_not_exists
    file_contains file_not_contains
  ).freeze

  state :root do
    # Comments
    rule %r/#.*$/, Comment::Single

    # Punctuation and operators
    rule %r/[{}=]/, Punctuation

    # Use the arrays defined above to find and tokenise keywords
    rule %r/\b(#{KEYWORD_DECLARATION.join('|')})\b/, Keyword::Declaration
    rule %r/\b(#{KEYWORD_STEP.join('|')})\b/, Keyword
    rule %r/\b(#{BUILTIN_LITERAL.join('|')})\b/, Name::Builtin
    rule %r/\b(#{COMMANDS.join('|')})\b/, Name::Function
    rule %r/\b(#{ASSERTIONS.join('|')})\b/, Name::Decorator # Often a green color

    # Variable usage
    rule %r/\$\{.*?}/, Name::Variable

    # Strings in double quotes
    rule %r/"/, Str::Double, :string

    # Numbers and time values
    rule %r/\b\d+s?\b/, Num

    # Whitespace
    rule %r/\s+/, Text::Whitespace
  end

  state :string do
    rule %r/"/, Str::Double, :pop!
    rule %r/\$\{.*?}/, Name::Variable # Interpolation
    rule %r/[^"]+/, Str::Double
  end
end
