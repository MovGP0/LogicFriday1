namespace LogicFriday1.Services;

public static class LogicEquationParser
{
    private const int MaximumInputCount = 16;
    private const int MaximumOutputCount = 16;
    private const int MaximumVariableNameLength = 8;

    public static LogicEquationParseResult Parse(string text)
    {
        ArgumentNullException.ThrowIfNull(text);

        var trimmedText = text.Trim();
        if (trimmedText.Length == 0)
        {
            throw new LogicEquationParseException("Enter a logic equation.");
        }

        if (!trimmedText.EndsWith(';'))
        {
            throw new LogicEquationParseException("An equation must be terminated with a semicolon.");
        }

        var equations = SplitEquations(trimmedText);
        if (equations.Count == 0)
        {
            throw new LogicEquationParseException("Enter a logic equation.");
        }

        if (equations.Count > MaximumOutputCount)
        {
            throw new LogicEquationParseException($"Logic equations are limited to {MaximumOutputCount} outputs.");
        }

        var assignments = equations.Select(ParseAssignment).ToArray();
        var duplicateOutputName = assignments
            .GroupBy(static assignment => assignment.OutputName, StringComparer.OrdinalIgnoreCase)
            .FirstOrDefault(static group => group.Count() > 1)
            ?.Key;
        if (duplicateOutputName is not null)
        {
            throw new LogicEquationParseException($"Duplicate output variable '{duplicateOutputName}'.");
        }

        var outputNames = assignments.Select(static assignment => assignment.OutputName).ToArray();
        var outputNameSet = new HashSet<string>(outputNames, StringComparer.OrdinalIgnoreCase);
        var inputNames = new List<string>();
        var inputNameSet = new HashSet<string>(StringComparer.OrdinalIgnoreCase);
        var parsedEquations = new List<ParsedEquation>();

        foreach (var assignment in assignments)
        {
            var parser = new ExpressionParser(assignment.ExpressionText);
            var expression = parser.ParseExpression();
            parsedEquations.Add(new ParsedEquation(assignment.OutputName, expression));

            foreach (var variableName in expression.VariableNames)
            {
                if (outputNameSet.Contains(variableName) || inputNameSet.Contains(variableName))
                {
                    continue;
                }

                inputNameSet.Add(variableName);
                inputNames.Add(variableName);
            }
        }

        if (inputNames.Count > MaximumInputCount)
        {
            throw new LogicEquationParseException($"Logic equations are limited to {MaximumInputCount} inputs.");
        }

        var outputValues = EvaluateOutputValues(inputNames, parsedEquations);
        return new LogicEquationParseResult(
            [.. inputNames],
            outputNames,
            outputValues,
            NormalizeEquationText(parsedEquations));
    }

    private static RawEquation ParseAssignment(string equationText)
    {
        var equalsIndex = equationText.IndexOf('=');
        if (equalsIndex < 0)
        {
            throw new LogicEquationParseException("You must assign an output variable with '='.");
        }

        if (equationText.IndexOf('=', equalsIndex + 1) >= 0)
        {
            throw new LogicEquationParseException("Syntax error in equation.");
        }

        var outputName = equationText[..equalsIndex].Trim();
        ValidateVariableName(outputName);

        var expressionText = equationText[(equalsIndex + 1)..].Trim();
        if (expressionText.Length == 0)
        {
            throw new LogicEquationParseException("Syntax error in equation.");
        }

        return new RawEquation(outputName, expressionText);
    }

    private static List<string> SplitEquations(string text)
    {
        var equations = new List<string>();
        var start = 0;
        var depth = 0;

        for (var index = 0; index < text.Length; index++)
        {
            switch (text[index])
            {
                case '(':
                    depth++;
                    break;
                case ')':
                    depth--;
                    if (depth < 0)
                    {
                        throw new LogicEquationParseException("Unmatched ')' in input.");
                    }

                    break;
                case ';':
                    if (depth != 0)
                    {
                        throw new LogicEquationParseException("Unmatched '(' in input.");
                    }

                    var equation = text[start..index].Trim();
                    if (equation.Length > 0)
                    {
                        equations.Add(equation);
                    }

                    start = index + 1;
                    break;
            }
        }

        return equations;
    }

    private static string NormalizeEquationText(IEnumerable<ParsedEquation> equations)
    {
        return string.Join(
            Environment.NewLine,
            equations.Select(static equation => $"{equation.OutputName} = {equation.Expression};"));
    }

    private static string[][] EvaluateOutputValues(
        IReadOnlyList<string> inputNames,
        IReadOnlyList<ParsedEquation> equations)
    {
        var rowCount = 1 << inputNames.Count;
        var outputValues = new string[rowCount][];

        for (var rowIndex = 0; rowIndex < rowCount; rowIndex++)
        {
            var values = new Dictionary<string, bool>(StringComparer.OrdinalIgnoreCase);
            for (var inputIndex = 0; inputIndex < inputNames.Count; inputIndex++)
            {
                var bitOffset = inputNames.Count - inputIndex - 1;
                values[inputNames[inputIndex]] = ((rowIndex >> bitOffset) & 1) == 1;
            }

            outputValues[rowIndex] = equations
                .Select(equation => equation.Expression.Evaluate(values) ? "1" : "0")
                .ToArray();
        }

        return outputValues;
    }

    private static void ValidateVariableName(string name)
    {
        if (name.Length == 0)
        {
            throw new LogicEquationParseException("Function name is undefined.");
        }

        if (name.Length > MaximumVariableNameLength)
        {
            throw new LogicEquationParseException($"Variable names are limited to {MaximumVariableNameLength} characters.");
        }

        if (!IsVariableStart(name[0]))
        {
            throw new LogicEquationParseException(
                $"Variable names must begin with a letter or underscore. Illegal char: '{name[0]}'");
        }

        foreach (var character in name)
        {
            if (!IsVariablePart(character))
            {
                throw new LogicEquationParseException($"Illegal character: '{character}'");
            }
        }
    }

    private static bool IsVariableStart(char value)
    {
        return char.IsLetter(value) || value == '_';
    }

    private static bool IsVariablePart(char value)
    {
        return char.IsLetterOrDigit(value) || value is '.' or '_' or '[' or ']';
    }

    private static bool IsExpressionTermStart(Token token)
    {
        return token.Kind is TokenKind.Identifier or TokenKind.Constant or TokenKind.Not or TokenKind.OpenParen;
    }

    private sealed class ExpressionParser
    {
        private readonly IReadOnlyList<Token> _tokens;
        private int _position;

        public ExpressionParser(string text)
        {
            _tokens = Tokenize(text);
        }

        public ExpressionNode ParseExpression()
        {
            var expression = ParseOr();
            if (Current.Kind != TokenKind.End)
            {
                throw new LogicEquationParseException("Syntax error in equation.");
            }

            return expression;
        }

        private static IReadOnlyList<Token> Tokenize(string text)
        {
            var tokens = new List<Token>();
            for (var index = 0; index < text.Length;)
            {
                var character = text[index];
                if (char.IsWhiteSpace(character))
                {
                    index++;
                    continue;
                }

                switch (character)
                {
                    case '+':
                    case '|':
                        tokens.Add(new Token(TokenKind.Or, character.ToString()));
                        index++;
                        continue;
                    case '*':
                    case '&':
                        tokens.Add(new Token(TokenKind.And, character.ToString()));
                        index++;
                        continue;
                    case '!':
                    case '~':
                        tokens.Add(new Token(TokenKind.Not, character.ToString()));
                        index++;
                        continue;
                    case '\'':
                        tokens.Add(new Token(TokenKind.PostfixNot, character.ToString()));
                        index++;
                        continue;
                    case '(':
                        tokens.Add(new Token(TokenKind.OpenParen, character.ToString()));
                        index++;
                        continue;
                    case ')':
                        tokens.Add(new Token(TokenKind.CloseParen, character.ToString()));
                        index++;
                        continue;
                    case '0':
                    case '1':
                        tokens.Add(new Token(TokenKind.Constant, character.ToString()));
                        index++;
                        continue;
                }

                if (!IsVariableStart(character))
                {
                    throw new LogicEquationParseException($"Illegal character: '{character}'");
                }

                var start = index;
                index++;
                while (index < text.Length && IsVariablePart(text[index]))
                {
                    index++;
                }

                var identifier = text[start..index];
                ValidateVariableName(identifier);
                tokens.Add(identifier.Equals("not", StringComparison.OrdinalIgnoreCase)
                    ? new Token(TokenKind.Not, identifier)
                    : new Token(TokenKind.Identifier, identifier));
            }

            tokens.Add(new Token(TokenKind.End, ""));
            return tokens;
        }

        private ExpressionNode ParseOr()
        {
            var left = ParseAnd();
            while (Match(TokenKind.Or))
            {
                left = new OrExpressionNode(left, ParseAnd());
            }

            return left;
        }

        private ExpressionNode ParseAnd()
        {
            var left = ParseUnary();
            while (true)
            {
                if (Match(TokenKind.And))
                {
                    left = new AndExpressionNode(left, ParseUnary());
                    continue;
                }

                if (!IsExpressionTermStart(Current))
                {
                    return left;
                }

                left = new AndExpressionNode(left, ParseUnary());
            }
        }

        private ExpressionNode ParseUnary()
        {
            if (Match(TokenKind.Not))
            {
                return new NotExpressionNode(ParseUnary());
            }

            var node = ParsePrimary();
            while (Match(TokenKind.PostfixNot))
            {
                node = new NotExpressionNode(node);
            }

            return node;
        }

        private ExpressionNode ParsePrimary()
        {
            if (Match(TokenKind.Constant, out var constant))
            {
                return new ConstantExpressionNode(constant.Text == "1");
            }

            if (Match(TokenKind.Identifier, out var identifier))
            {
                return new VariableExpressionNode(identifier.Text);
            }

            if (Match(TokenKind.OpenParen))
            {
                var expression = ParseOr();
                if (!Match(TokenKind.CloseParen))
                {
                    throw new LogicEquationParseException("Unmatched '(' in input.");
                }

                return expression;
            }

            throw new LogicEquationParseException("Syntax error in equation.");
        }

        private Token Current => _tokens[_position];

        private bool Match(TokenKind kind)
        {
            if (Current.Kind != kind)
            {
                return false;
            }

            _position++;
            return true;
        }

        private bool Match(TokenKind kind, out Token token)
        {
            token = Current;
            if (Current.Kind != kind)
            {
                return false;
            }

            _position++;
            return true;
        }
    }

    private abstract record ExpressionNode
    {
        public abstract IEnumerable<string> VariableNames { get; }
        public abstract bool Evaluate(IReadOnlyDictionary<string, bool> values);
    }

    private sealed record ConstantExpressionNode(bool Value) : ExpressionNode
    {
        public override IEnumerable<string> VariableNames => [];
        public override bool Evaluate(IReadOnlyDictionary<string, bool> values) => Value;
        public override string ToString() => Value ? "1" : "0";
    }

    private sealed record VariableExpressionNode(string Name) : ExpressionNode
    {
        public override IEnumerable<string> VariableNames => [Name];

        public override bool Evaluate(IReadOnlyDictionary<string, bool> values)
        {
            return values.TryGetValue(Name, out var value) && value;
        }

        public override string ToString() => Name;
    }

    private sealed record NotExpressionNode(ExpressionNode Operand) : ExpressionNode
    {
        public override IEnumerable<string> VariableNames => Operand.VariableNames;
        public override bool Evaluate(IReadOnlyDictionary<string, bool> values) => !Operand.Evaluate(values);

        public override string ToString()
        {
            return Operand is VariableExpressionNode or ConstantExpressionNode
                ? $"{Operand}'"
                : $"({Operand})'";
        }
    }

    private sealed record AndExpressionNode(ExpressionNode Left, ExpressionNode Right) : ExpressionNode
    {
        public override IEnumerable<string> VariableNames => Left.VariableNames.Concat(Right.VariableNames);
        public override bool Evaluate(IReadOnlyDictionary<string, bool> values) => Left.Evaluate(values) && Right.Evaluate(values);

        public override string ToString()
        {
            return $"{FormatTerm(Left)} {FormatTerm(Right)}";
        }

        private static string FormatTerm(ExpressionNode node)
        {
            return node is OrExpressionNode ? $"({node})" : node.ToString();
        }
    }

    private sealed record OrExpressionNode(ExpressionNode Left, ExpressionNode Right) : ExpressionNode
    {
        public override IEnumerable<string> VariableNames => Left.VariableNames.Concat(Right.VariableNames);
        public override bool Evaluate(IReadOnlyDictionary<string, bool> values) => Left.Evaluate(values) || Right.Evaluate(values);
        public override string ToString() => $"{Left} + {Right}";
    }

    private sealed record ParsedEquation(string OutputName, ExpressionNode Expression);

    private sealed record RawEquation(string OutputName, string ExpressionText);

    private readonly record struct Token(TokenKind Kind, string Text);

    private enum TokenKind
    {
        Identifier,
        Constant,
        Or,
        And,
        Not,
        PostfixNot,
        OpenParen,
        CloseParen,
        End
    }
}
