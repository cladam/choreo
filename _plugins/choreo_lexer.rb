# _plugins/choreo_lexer.rb

require 'rouge'

class Choreo < Rouge::RegexLexer
  title "Choreo"
  desc "A DSL for behavior-driven testing (github.com/cladam/choreo)"
  tag 'choreo'
  filenames '*.chor'
  mimetypes 'text/x-choreo'

  state :root do
    # Comments
    rule %r/#.*$/, Comment::Single

    # Keywords for main blocks (e.g., feature, actors)
    rule %r/^(feature|actors|settings|background|scenario|after|var)\b/, Keyword::Declaration, :main_block

    # Keywords for test steps
    rule %r/\b(given|when|then)\b:/, Keyword, :pop!

    # The 'test' keyword
    rule %r/\b(test)\b/, Keyword, :main_block
    
    # Built-in actors and literal values
    rule %r/\b(Web|Terminal|FileSystem|true|false)\b/, Name::Builtin

    # Commands and Assertions
    rule %r/\b([a-zA-Z_][a-zA-Z0-9_]+)\b(?=\s*")/, Name::Function # Catches functions before a string
    rule %r/\b([a-zA-Z_][a-zA-Z0-9_]+)\b(?=\s*\$\{)/, Name::Function # Catches functions before a variable
    rule %r/\b([a-zA-Z_][a-zA-Z0-9_]+)\b(?=\s*\d+s?)/, Name::Function # Catches functions before a number/time
    rule %r/\b([a-zA-Z_][a-zA-Z0-9_]+)\b$/, Name::Function # Catches functions at the end of a line

    # Punctuation and operators
    rule %r/[{}=:]|>=/, Punctuation

    # Strings in double quotes
    rule %r/"/, Str::Double, :string

    # Numbers and time values
    rule %r/\b\d+s?\b/, Num
    
    # Everything else is plain text
    rule %r/\s+/, Text::Whitespace
    rule %r/.+?/, Text
  end

  # State for handling the rest of the line after a main keyword
  state :main_block do
    rule %r/\n/, Text::Whitespace, :pop!
    rule %r/"/, Str::Double, :string # Handles strings in declarations
    rule %r/.+?/, Text
  end

  # State for handling strings
  state :string do
    rule %r/"/, Str::Double, :pop!
    rule %r/\$\{.*?}/, Name::Variable # Interpolation
    rule %r/[^"]+/, Str::Double
  end
end
