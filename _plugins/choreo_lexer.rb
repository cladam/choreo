# _plugins/choreo_lexer.rb

require 'rouge'

class Choreo < Rouge::RegexLexer
  title "Choreo"
  desc "A DSL for behavior-driven testing (github.com/cladam/choreo)"
  tag 'choreo'
  filenames '*.chor'
  mimetypes 'text/x-choreo'

  # --- Word Lists ---
  KEYWORD_DECLARATION = %w(
    feature actors settings background scenario after test var
  ).freeze

  KEYWORD_STEP = %w(
    given when then
  ).freeze

  BUILTIN_LITERAL = %w(
    Web Terminal FileSystem true false
  ).freeze

  COMMANDS_AND_ASSERTIONS = %w(
    wait set_header set_cookie http_get http_post clear_header clear_cookie
    run type wait_for_text create_file delete_file append_to_file
    response_status_is response_time_is_below response_header_is response_body_is
    response_body_contains expect_exit_code stdout_contains stderr_contains
    stdout_not_contains stderr_not_contains file_exists file_not_exists
    file_contains file_not_contains
  ).freeze

  # --- Main Lexer States ---
  state :root do
    rule %r/\s+/m, Text::Whitespace
    rule %r/#.*$/, Comment::Single

    # These keywords are matched first, before general text
    prepended :keywords

    # Punctuation and operators
    rule %r/[{}=:]|>=/, Punctuation

    # Strings, which can contain variables
    rule %r/"/, Str::Double, :string

    # Numbers and time values
    rule %r/\b\d+s?\b/, Num
    
    # General text, variable names, test names etc.
    rule %r/[a-zA-Z_][a-zA-Z0-9_]*/, Text
  end
  
  # The 'keywords' state is checked before the rest of the 'root' state
  state :keywords do
    rule %r/\b(#{KEYWORD_DECLARATION.join('|')})\b/, Keyword::Declaration
    rule %r/\b(#{KEYWORD_STEP.join('|')})\b(?=:)/, Keyword
    rule %r/\b(#{BUILTIN_LITERAL.join('|')})\b/, Name::Builtin
    rule %r/\b(#{COMMANDS_AND_ASSERTIONS.join('|')})\b/, Name::Function
  end

  # State for handling content inside strings
  state :string do
    rule %r/"/, Str::Double, :pop!
    rule %r/\$\{.*?}/, Name::Variable # Interpolation
    rule %r/[^"]+/, Str::Double
  end
end
