start
  = Grammar

Grammar
  = Definition*

Divider
  = ":=" / "::="

OpenParens
  = "'('"

CloseParens
  = "')'"

Definition
  = lhs:LHS _ Divider _ rhs:RHS _ { return { "lhs": lhs, "rhs": rhs } }

LHS
  = Identifier

RHS
  = Alternatives / Function

NIdentifier
  = ZeroOrMore / ZeroOrOne / Identifier

Identifier
  = [a-zA-Z]+ { return text(); }

ZeroOrMore
  = "{" _ ident:Identifier _ "}" {
    return {
      type: "zeroOrMore",
      name: ident,
    };
  }

ZeroOrOne
  = "[" _ ident:Identifier _ "]" {
    return {
      type: "zeroOrOne",
      name: ident,
    };
  }

IdentifierList
  = idents:(NIdentifier _)+ {
    return idents.map(match => match[0]);
  }

Literal
  = "'" ident:Identifier "'" { return ident; }

Alternatives
  = alternatives:Alternative+ {
    if (alternatives.length === 1) {
      return alternatives[0];
    }
    return alternatives;
  }

Alternative
  = ident:(NIdentifier)+ AlternativeDivider? { return ident[0]; }

AlternativeDivider
  = _ "|" _?

Function
  = fnName:Literal _ OpenParens _ params:IdentifierList CloseParens {
    return {
      type: 'function',
      name: fnName,
      params: params
    };
  }

_ "whitespace"
  = [ \t\n\r]*

