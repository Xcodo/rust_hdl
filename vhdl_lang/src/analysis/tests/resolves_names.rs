// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this file,
// You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) 2019, Olof Kraigher olof.kraigher@gmail.com

use super::*;

#[test]
fn resolves_names_in_object_decl_init_expressions() {
    let mut builder = LibraryBuilder::new();
    let code = builder.code(
        "libname",
        "
package pkg is
  constant c0 : natural := 0;
  constant c1 : natural := c0;
  constant c2 : natural := missing;
  constant c3 : natural := 1 + missing;
  constant c4 : natural := - missing;
end package;",
    );

    let (root, diagnostics) = builder.get_analyzed_root();
    check_diagnostics(
        diagnostics,
        (1..=3).map(|idx| missing(&code, "missing", idx)).collect(),
    );

    // Goto declaration from declaration
    assert_eq!(
        root.search_reference(code.source(), code.s("c0", 1).end()),
        Some(code.s("c0", 1).pos())
    );

    // Goto declaration from reference
    assert_eq!(
        root.search_reference(code.source(), code.s("c0", 2).end()),
        Some(code.s("c0", 1).pos())
    );
}

#[test]
fn resolves_names_in_iface_object_decl_init_expressions() {
    check_missing(
        "
package pkg is
  function foo(constant c2 : natural := missing) return natural;
end package;",
    );
}

#[test]
fn search_names_in_iface_object_decl_init_expressions() {
    check_search_reference(
        "
    package pkg is
      constant decl : natural := 0;
      function foo(constant c2 : natural := decl) return natural;
    end package;",
    );
}

#[test]
fn subprogram_parameters_are_visible_in_body() {
    check_missing(
        "
package pkg is
end package;

package body pkg is
  function foo(c0 : natural) return natural is
     constant c1 : natural := c0;
     constant c2 : natural := missing;
  begin
     return c1;
  end;
end package body;
",
    );
}

/// Check that at least the prefix is resolved also for names which are not purely selected names
#[test]
fn resolves_names_for_prefix_of_non_selected() {
    let mut builder = LibraryBuilder::new();
    let code = builder.code(
        "libname",
        "
package pkg is
  type arr_t is array (natural range 0 to 1) of integer_vector(0 to 1);
  constant c0 : integer_vector(0 to 1) := (0,1);
  constant c1 : natural := c0(0);
  constant c2 : natural := missing(0);
  constant c3 : integer_vector(0 to 1) := c0(0 to 1);
  constant c4 : integer_vector(0 to 1) := missing(0 to 1);
  constant c5 : arr_t := (0 => (0,1), 1 => (2, 3));

  -- This was a bug at one point
  constant c6 : natural := c5(0)'length;
  constant c7 : natural := missing(0)'length;

  -- This was also a bug at one point
  type rec_t is record
      field : natural;
  end record;
  type rec_arr_t is array (natural range <>) of rec_t;
  constant c8 : rec_arr_t(0 to 0) := (0 => (field => 0));
  constant c9 : natural := c8(0).field;
  constant ca : natural := missing(0).field;
  constant cb : rec_t := (field => 0);
  constant cc : natural := cb.field;
  constant cd : natural := missing.field;

end package;",
    );

    let (root, diagnostics) = builder.get_analyzed_root();
    check_diagnostics(
        diagnostics,
        (1..=5).map(|idx| missing(&code, "missing", idx)).collect(),
    );

    // Goto declaration from reference that is prefix of indexed/function call
    assert_eq!(
        root.search_reference(code.source(), code.s("c0", 2).end()),
        Some(code.s("c0", 1).pos())
    );

    // Goto declaration from reference that is prefix of slice
    assert_eq!(
        root.search_reference(code.source(), code.s("c0", 3).end()),
        Some(code.s("c0", 1).pos())
    );
}

#[test]
fn labels_are_visible() {
    let mut builder = LibraryBuilder::new();
    let code = builder.code(
        "libname",
        "
entity ent is
end entity;

architecture a of ent is
begin
  lab1 : process
     constant name1 : string := lab1'instance_name;
     constant dummy : string := missing'instance_name;
  begin
  end process;

  lab2 : block is
     constant name1 : string := lab1'instance_name;
     constant name2 : string := lab2'instance_name;
     constant dummy : string := missing'instance_name;
  begin
  end block;
end architecture;
",
    );

    let (root, diagnostics) = builder.get_analyzed_root();
    check_diagnostics(
        diagnostics,
        (1..=2).map(|idx| missing(&code, "missing", idx)).collect(),
    );

    for i in 1..=3 {
        assert_eq!(
            root.search_reference(code.source(), code.s("lab1", i).end()),
            Some(code.s("lab1", 1).pos())
        );
    }

    for i in 1..=2 {
        assert_eq!(
            root.search_reference(code.source(), code.s("lab2", i).end()),
            Some(code.s("lab2", 1).pos())
        );
    }
}

#[test]
fn resolves_names_in_discrete_ranges() {
    check_missing(
        "
package pkg is
  type arr_t is array (natural range missing to missing) of natural;
  type arr2_t is array (missing to missing) of natural;
  type arr3_t is array (missing'range) of natural;
end package;
",
    );
}

#[test]
fn search_names_in_discrete_ranges() {
    check_search_reference(
        "
package pkg is
  constant decl : natural := 0;
  type arr_t is array (natural range decl to decl) of natural;
  type arr2_t is array (decl to decl) of natural;
  type arr3_t is array (decl'range) of natural;
end package;
",
    );
}

#[test]
fn resolves_names_in_subtype_constraints() {
    // @TODO check for missing fields in record element constraints
    check_missing(
        "
package pkg is
  subtype sub1_t is integer_vector(missing to missing);
  subtype sub2_t is integer range missing to missing;

  type rec_t is record
    field : integer_vector;
  end record;

  subtype sub3_t is rec_t(field(missing to missing));

  type uarr_t is array (natural range <>) of integer_vector;
  subtype sub4_t is uarr_t(open)(missing to missing);

end package;
",
    );
}

#[test]
fn search_names_in_subtype_constraints() {
    check_search_reference(
        "
package pkg is
  constant decl : natural := 0;
  subtype sub1_t is integer_vector(decl to decl);
  subtype sub2_t is integer range decl to decl;

  type rec_t is record
    field : integer_vector;
  end record;

  subtype sub3_t is rec_t(field(decl to decl));

  type uarr_t is array (natural range <>) of integer_vector;
  subtype sub4_t is uarr_t(open)(decl to decl);

end package;
",
    );
}

#[test]
fn search_names_in_integer_type_declaration_ranges() {
    check_search_reference(
        "
package pkg is
  constant decl : natural := 0;
  type int_t is range 0 to decl;
end package;
",
    );
}

#[test]
fn search_names_in_integer_type_declaration() {
    check_search_reference(
        "
package pkg is
  type decl is range 0 to 3;
  constant foo : decl := 0;
end package;
",
    );
}

#[test]
fn search_names_in_file_type_declaration() {
    check_search_reference(
        "
package pkg is
  subtype decl is character;
  type foo is file of decl;
end package;
",
    );
}

#[test]
fn resolves_names_in_inside_names() {
    check_missing(
        "
package pkg is
end package;

package body pkg is
  type arr2d_t is array (natural range 0 to 1, natural range 0 to 1) of natural;

  function fun(a, b : natural) return natural is
  begin
    return 0;
  end;

  function fun2 return natural is
     -- Function call
     constant c : natural := fun(a => missing, b => missing);
     -- Function call
     constant c2 : natural := fun(missing, missing);

     variable arr : arr2d_t;
     -- Indexed
     constant c3 : natural := arr(missing, missing);

     -- Slice
     constant vec : integer_vector(0 to 1) := (0, 1);
     constant c4 : integer_vector(0 to 1) := vec(missing to missing);

     constant c5 : natural := missing'val(0);
     constant c6 : boolean := boolean'val(missing);
  begin
  end;

end package body;
",
    );
}

#[test]
fn search_names_in_inside_names() {
    check_search_reference(
        "
package pkg is
end package;

package body pkg is
  constant decl : natural := 0;
  type arr2d_t is array (natural range 0 to 1, natural range 0 to 1) of natural;

  function fun(a, b : natural) return natural is
  begin
    return 0;
  end;

  function fun2 return natural is
     -- Function call
     constant c : natural := fun(a => decl, b => decl);
     -- Function call
     constant c2 : natural := fun(decl, decl);

     variable arr : arr2d_t;
     -- Indexed
     constant c3 : natural := arr(decl, decl);

     -- Slice
     constant vec : integer_vector(0 to 1) := (0, 1);
     constant c4 : integer_vector(0 to 1) := vec(decl to decl);

     constant c5 : string := decl'simple_name;
     constant c6 : boolean := boolean'val(decl);
  begin
  end;

end package body;
",
    );
}

#[test]
fn resolves_names_in_aggregates() {
    check_missing(
        "
package pkg is
  -- Named
  constant c0 : integer_vector(0 to 0) := (0 => missing);
  constant c1 : integer_vector(0 to 0) := (missing to missing => 0);

  -- Positional
  constant c2 : integer_vector(0 to 1) := (missing, missing);
end package;
",
    );
}

#[test]
fn search_names_in_aggregates() {
    check_search_reference(
        "
package pkg is
  constant decl : natural := 0;

  -- Named
  constant c0 : integer_vector(0 to 0) := (0 => decl);
  constant c1 : integer_vector(0 to 0) := (decl to decl => 0);

  -- Positional
  constant c2 : integer_vector(0 to 1) := (decl, decl);
end package;
",
    );
}

#[test]
fn resolves_names_in_qualified_expr() {
    check_missing(
        "
package pkg is
  -- Named
  constant c0 : missing := missing'(1 + missing);
end package;
",
    );
}

#[test]
fn search_names_in_qualified_expr() {
    check_search_reference(
        "
package pkg is
  type decl is range 0 to 1;
  -- Named
  constant c0 : decl := decl'(decl'(0));
end package;
",
    );
}

#[test]
fn resolves_names_in_allocators() {
    check_missing(
        "
package pkg is
end package;

package body pkg is
    procedure p is
       type acc_t is access integer_vector;
       -- Qualified
       variable ptr0 : acc_t := new integer_vector'(missing, missing);
       -- Subtype
       variable ptr1 : acc_t := new integer_vector(missing to missing);
    begin
    end procedure;
end package body;
",
    );
}

#[test]
fn search_names_in_allocators() {
    check_search_reference(
        "
package pkg is
end package;

package body pkg is
    constant decl : natural := 0;
    procedure p is
       type acc_t is access integer_vector;
       -- Qualified
       variable ptr0 : acc_t := new integer_vector'(decl, decl);
       -- Subtype
       variable ptr1 : acc_t := new integer_vector(decl to decl);
    begin
    end procedure;
end package body;
",
    );
}

#[test]
fn resolves_names_in_sequential_statements() {
    check_missing(
        "
package pkg is
end package;

package body pkg is
    function f2(c : natural) return natural is
    begin
      return c;
    end;

    function f return natural is
    begin
       -- Variable assignment
       missing := missing;
       missing := missing when missing else missing;
       with missing select
         missing := missing when missing,
                    missing when others;

       -- Procedure call
       missing;
       missing(missing);

       -- If statement
       if missing = 1 then
         missing;
       elsif missing = 2 then
         missing;
       else
         missing;
       end if;

       -- Loops
       for i in missing to missing loop
         f2(i); -- Index is defined
         missing;
       end loop;

       loop
         missing;
         next when missing;
         exit when missing;
       end loop;

       while missing loop
         missing;
       end loop;

       -- Case
       case missing is
         when missing =>
           missing;
         when missing to missing =>
           missing;
       end case;

       report missing severity missing;
       assert missing report missing severity missing;

       -- Return
       return missing;
    end;
end package body;
",
    );
}

#[test]
fn search_names_in_sequential_statements() {
    check_search_reference(
        "
package pkg is
end package;

package body pkg is
  procedure proc(c : natural) is
  begin
  end;

  function f return natural is
    variable decl : natural := 0;
  begin
    -- Variable assignment
    decl := decl;
    decl := decl when decl = 0 else decl;
    with decl select
      decl := decl when decl,
              decl when others;

    -- Procedure call
    proc(decl);


    -- If statement
    if decl = 1 then
      proc(decl);
    elsif decl = 2 then
      proc(decl);
    else
      proc(decl);
    end if;

    -- Loops
    for i in decl to decl loop
      proc(decl);
    end loop;

    loop
      proc(decl);
      next when decl = 0;
      exit when decl = 0;
    end loop;

    while decl = 0 loop
      proc(decl);
    end loop;

    -- Case
    case decl is
      when decl =>
        proc(decl);
      when decl to decl =>
        proc(decl);
    end case;

    report natural'image(decl) severity severity_level'val(decl);
    assert decl = 0 report natural'image(decl) severity severity_level'val(decl);

    -- Return
    return decl;
  end;
end package body;
",
    );
}

#[test]
fn check_missing_in_process_statements() {
    check_missing(
        "
entity ent is
end entity;

architecture a of ent is
begin
  main : process(missing) is
  begin
    wait on missing until missing = 0 ns for missing;
    missing <= missing after missing;
    missing <= force missing;
    missing <= release;
    with missing select
       missing <= missing when missing,
                  missing when others;

  end process;
end architecture;
",
    );
}

#[test]
fn search_in_process_statements() {
    check_search_reference(
        "
entity ent is
end entity;

architecture a of ent is
  signal decl : time;
begin
  main : process (decl) is
  begin
    wait on decl until decl = 0 ns for decl;
    decl <= decl after decl;
    decl <= force decl;
    decl <= release;
    with decl select
       decl <= decl when decl,
               decl when others;
  end process;
end architecture;
",
    );
}

#[test]
fn search_in_aggregate_target() {
    check_search_reference(
        "
entity ent is
end entity;

architecture a of ent is
  signal decl : natural;
begin
  main : process is
  begin
   (0 => decl) := (0 => decl);
  end process;
end architecture;
",
    );
}

#[test]
fn check_missing_in_instantiations() {
    check_missing(
        "
entity ename is
  generic (g : natural);
  port (s : natural);
end entity;

entity ent is
end entity;

architecture a of ent is
  component comp is
    generic (g : natural);
    port (s : natural);
  end component;
begin
  inst: entity work.ename
    generic map (
      g => missing)
    port map (
      s => missing);

  inst2: component comp
    generic map (
      g => missing)
    port map (
      s => missing);
end architecture;
",
    );
}

#[test]
fn check_search_in_instantiations() {
    check_search_reference(
        "
entity ename is
  generic (g : natural);
  port (s : natural);
end entity;

entity ent is
end entity;

architecture a of ent is
  component comp is
    generic (g : natural);
    port (s : natural);
  end component;

  constant decl : natural := 0;
begin
  inst: entity work.ename
    generic map (
      g => decl)
    port map (
      s => decl);

  inst2: component comp
    generic map (
      g => decl)
    port map (
      s => decl);
end architecture;
",
    );
}

#[test]
fn package_name_visible_in_header_and_body() {
    check_code_with_no_diagnostics(
        "
package pkg is
  constant name1 : string := pkg'instance_name;
end package;

package body pkg is
  constant name2 : string := pkg'instance_name;
end package body;
",
    );
}

#[test]
fn search_package_name_in_header_and_body() {
    check_search_reference(
        "
package decl is
  constant name1 : string := decl'instance_name;
end package;

package body decl is
  constant name2 : string := decl'instance_name;
end package body;
",
    );
}

#[test]
fn unit_name_visible_in_entity_architecture() {
    check_code_with_no_diagnostics(
        "
entity ent is
begin
  p1 : process is
  begin
    report ent'instance_name;
  end process;
end entity;

architecture a of ent is
begin
  p2 : process is
  begin
    report ent'instance_name;
    report a'instance_name;
  end process;
end;
",
    );
}

#[test]
fn search_entity_name_in_entity() {
    check_search_reference(
        "
entity decl is
end entity;

architecture a of decl is
begin
  main : process is
  begin
    report decl'instance_name;
  end process;
end architecture;

",
    );
}

#[test]
fn resolves_names_in_concurrent_statements() {
    check_missing(
        "
entity ent is
end entity;

architecture a of ent is
begin
  missing <= missing;
  missing <= missing when missing else missing;
  with missing select
     missing <= missing when missing,
                missing when others;
  missing(missing);
  assert missing report missing severity missing;
end architecture;
",
    );
}

#[test]
fn search_names_in_concurrent_statements() {
    check_search_reference(
        "
entity ent is
end entity;

architecture a of ent is
  procedure proc(signal sig : in natural) is
  begin
  end;

  signal decl : natural := 0;
begin
  decl <= decl;
  decl <= decl when decl = 0 else decl;
  with decl select
     decl <= decl when decl,
             decl when others;
  proc(decl);
  assert decl = 0 report decl'instance_name severity severity_level'val(decl);
end architecture;
",
    );
}

#[test]
fn search_for_loop_index() {
    check_search_reference(
        "
entity ent is
end entity;

architecture a of ent is
begin
main : process is
begin
 for decl in 0 to 3 loop
     report integer'image(decl);
 end loop;
end process;
end architecture;

",
    );
}

#[test]
fn search_for_generate_index_range() {
    check_search_reference(
        "
entity ent is
end entity;

architecture a of ent is
  constant decl : natural := 0;
begin
 gen: for i in decl to decl+3 generate
 end generate;
end architecture;
",
    );
}

#[test]
fn search_for_generate_index() {
    check_search_reference(
        "
entity ent is
end entity;

architecture a of ent is
  signal foo : integer_vector(0 to 3);
begin
 gen: for decl in foo'range generate
    foo(decl) <= 0;
 end generate;
end architecture;

",
    );
}

#[test]
fn search_if_generate_conditions() {
    check_search_reference(
        "
entity ent is
end entity;

architecture a of ent is
  constant decl : natural := 0;
  signal foo : natural;
begin
 gen: if decl = 0 generate
   foo <= decl;
 else generate
   foo <= decl + 1;
 end generate;
end architecture;
",
    );
}

#[test]
fn search_generate_alternate_labels() {
    check_search_reference(
        "
entity ent is
end entity;

architecture a of ent is
begin
 gen: if decl: true generate
 end generate;
end architecture;
",
    );
}

#[test]
fn resolves_missing_name_in_alias() {
    check_missing(
        "
package pkg is
  alias a is missing[natural];
  alias b is maximum[missing];
end package;
",
    );
}

#[test]
fn search_name_in_alias() {
    check_search_reference(
        "
package pkg is
  type decl is (alpha, beta);
  function fun(arg : decl) return decl;
  procedure proc(arg : decl);
  alias a is fun[decl return decl];
  alias b is proc[decl];
  alias c is decl;
end package;
",
    );
}

#[test]
fn search_external_name() {
    check_search_reference(
        "
package pkg is
  type decl is (alpha, beta);
end package;

entity ent2 is
end entity;

use work.pkg.all;

architecture a of ent2 is
  signal foo : decl;
begin
end architecture;

entity ent is
end entity;

use work.pkg.all;

architecture a of ent is
  signal foo : decl;
begin
  inst : work.ent2;
  foo <= << signal inst.foo : decl >>;
end architecture;
",
    );
}

#[test]
fn block_names_are_visible() {
    check_code_with_no_diagnostics(
        "
entity ent is
  port (ent_in : integer);
end entity;

architecture a of ent is
  signal sig : integer;
begin
  blk: block (ent_in = 1) is
    generic( gen : integer := 0 );
    generic map ( gen => 1);
    port(
      prt_in : in integer := 0;
      prt_out : out integer := 0
    );
    port map (
      prt_in => ent_in + sig,
      prt_out => open
    );
  begin
    prt_out <= gen + prt_in + sig;
  end block;
end architecture;
",
    );
}

#[test]
fn error_on_signature_for_non_overloaded_alias() {
    let mut builder = LibraryBuilder::new();
    let code = builder.code(
        "libname",
        "
package pkg is
  type enum_t is (alpha, beta);
  alias alias_t is enum_t[return integer];
end package;
",
    );

    let diagnostics = builder.analyze();
    check_diagnostics(
        diagnostics,
        vec![Diagnostic::error(
            code.s1("[return integer]"),
            "Alias should only have a signature for subprograms and enum literals",
        )],
    );
}

#[test]
fn error_on_non_signature_for_overloaded_alias() {
    let mut builder = LibraryBuilder::new();
    let code = builder.code(
        "libname",
        "
package pkg is
end package;

package body pkg is
  procedure subpgm(arg: natural) is
  begin
  end;

  alias alias_t is subpgm;
end package body;
",
    );

    let diagnostics = builder.analyze();
    check_diagnostics(
        diagnostics,
        vec![Diagnostic::error(
            code.s("subpgm", 2),
            "Signature required for alias of subprogram and enum literals",
        )],
    );
}

#[test]
fn signatures_are_compared_with_base_type() {
    check_code_with_no_diagnostics(
        "
package pkg is
end package;

package body pkg is
  subtype sub_type is natural range 0 to 5;
  alias type_alias is sub_type;
  subtype sub_type2 is type_alias range 0 to 2;

  function subpgm(arg: sub_type2) return sub_type2 is
  begin
  end;

  alias alias1 is subpgm[integer, return integer];
  alias alias2 is subpgm[type_alias, return type_alias];
  alias alias3 is subpgm[sub_type, return sub_type];
  alias alias4 is subpgm[sub_type2, return sub_type2];
end package body;
",
    );
}

#[test]
fn can_goto_declaration_of_alias_with_signature() {
    let mut builder = LibraryBuilder::new();
    let code = builder.code(
        "libname",
        "
package pkg is
end package;

package body pkg is
  function subpgm(arg: natural) return natural is
  begin
  end;

  function subpgm(arg: boolean) return boolean is
  begin
  end;

  alias alias1 is subpgm[boolean, return boolean];
  alias alias2 is subpgm[integer, return integer];
end package body;
",
    );

    let (root, diagnostics) = builder.get_analyzed_root();
    check_no_diagnostics(&diagnostics);

    // Goto declaration from declaration
    assert_eq!(
        root.search_reference(code.source(), code.s("subpgm", 3).end()),
        Some(code.s("subpgm", 2).pos())
    );

    assert_eq!(
        root.search_reference(code.source(), code.s("subpgm", 4).end()),
        Some(code.s("subpgm", 1).pos())
    );
}

#[test]
fn overloaded_name_can_be_selected() {
    let mut builder = LibraryBuilder::new();
    builder.code(
        "libname",
        "
package pkg is
end package;

package body pkg is
  type rec_t is record
    f : natural;
  end record;

  function foo return rec_t is
    variable t : rec_t;
  begin
    return t;
  end function;

  procedure bar is
    variable s : natural;
  begin
    s := foo.f;
  end; 
end package body;
",
    );

    let diagnostics = builder.analyze();
    check_no_diagnostics(&diagnostics);
}

#[test]
fn record_fields_are_resolved() {
    let mut builder = LibraryBuilder::new();
    let code = builder.code(
        "libname",
        "
package pkg is

  type rec2_t is record
    field2 : natural;
  end record;

  type rec1_t is record
    field1 : rec2_t;
  end record;

  constant rec_val : rec1_t := (field1 => (field2 => 0));
  alias rec_alias is rec_val;
  
  -- Good
  constant a : rec2_t := rec_val.field1;
  constant b : natural := rec_val.field1.field2;
  constant c : rec2_t := rec_alias.field1;
  constant d : natural := rec_alias.field1.field2;
  
  -- Bad
  constant e : natural := rec_val.missing;
  constant f : natural := rec_val.field1.missing;
  constant g : natural := rec_alias.missing;
  constant h : natural := rec_alias.field1.missing;
end package;
",
    );

    let diagnostics = builder.analyze();
    check_diagnostics(
        diagnostics,
        vec![
            Diagnostic::error(
                code.s("missing", 1),
                "No declaration of 'missing' within record type 'rec1_t'",
            ),
            Diagnostic::error(
                code.s("missing", 2),
                "No declaration of 'missing' within record type 'rec2_t'",
            ),
            Diagnostic::error(
                code.s("missing", 3),
                "No declaration of 'missing' within record type 'rec1_t'",
            ),
            Diagnostic::error(
                code.s("missing", 4),
                "No declaration of 'missing' within record type 'rec2_t'",
            ),
        ],
    );
}

#[test]
fn find_all_references_of_record_field() {
    let mut builder = LibraryBuilder::new();
    let code = builder.code(
        "libname",
        "
entity ent is
end entity;

architecture a of ent is
  type rec_t is record
    field : natural;
  end record;

  signal sig : rec_t;
begin
  sig.field <= 1;
end architecture;",
    );

    let (root, diagnostics) = builder.get_analyzed_root();
    check_no_diagnostics(&diagnostics);

    let references = vec![code.s("field", 1).pos(), code.s("field", 2).pos()];

    assert_eq_unordered(
        &root.find_all_references(&code.s1("field").pos()),
        &references,
    );
}

#[test]
fn record_subtype_can_be_selected() {
    let mut builder = LibraryBuilder::new();
    let code = builder.code(
        "libname",
        "
package pkg is
end package;

package body pkg is
  type rec_t is record
    elem : integer_vector;
  end record;
  
  subtype sub_t is rec_t(elem(0 to 1));

  constant const1 : sub_t := (elem => (0, 1));
  
  -- Ok
  constant const2 : integer := const1.elem(0);

  -- Not ok
  constant const3 : integer := const1.missing;

end package body;
",
    );

    let diagnostics = builder.analyze();
    check_diagnostics(
        diagnostics,
        vec![Diagnostic::error(
            code.s("missing", 1),
            "No declaration of 'missing' within record type 'rec_t'",
        )],
    );
}

#[test]
fn acccess_type_of_record_can_be_selected() {
    let mut builder = LibraryBuilder::new();
    let code = builder.code(
        "libname",
        "
entity ent is
end;

architecture a of ent is
  type rec_t is record
    elem : integer_vector;
  end record;

  type rec_access_t is access rec_t;
begin

  main : process
      variable avar : rec_access_t := new rec_access_t'(elem => 0);
      variable v : natural;
  begin
     -- Ok
     v := avar.elem;
     -- Not ok
     v := avar.missing;
  end process;

end architecture;
",
    );

    let diagnostics = builder.analyze();
    check_diagnostics(
        diagnostics,
        vec![Diagnostic::error(
            code.s("missing", 1),
            "No declaration of 'missing' within record type 'rec_t'",
        )],
    );
}

#[test]
fn protected_type_can_be_selected() {
    let mut builder = LibraryBuilder::new();
    let code = builder.code(
        "libname",
        "
entity ent is
end;

architecture a of ent is
  type prot_t is protected
    function foo return natural;
  end protected;

  type prot_t is protected body
    function foo return natural is
    begin
      return 0;
    end;
  end protected body;

  shared variable pvar : prot_t;
begin

  main : process
      variable v : natural;
  begin
     -- Ok
     v := pvar.foo;

     -- Not ok
     v := pvar.missing;
  end process;

end architecture;
",
    );

    let diagnostics = builder.analyze();
    check_diagnostics(
        diagnostics,
        vec![Diagnostic::error(
            code.s("missing", 1),
            "No declaration of 'missing' within protected type 'prot_t'",
        )],
    );
}

#[test]
fn incomplete_acccess_type_of_record_can_be_selected() {
    let mut builder = LibraryBuilder::new();
    let code = builder.code(
        "libname",
        "
entity ent is
end;

architecture a of ent is

  type rec_t;
  type rec_access_t is access rec_t;

  type rec_t is record
    elem : natural;
    child : rec_access_t;
  end record;

begin

  main : process
      variable rvar : rec_t := (elem => 0, child => null);
      variable v : natural;
  begin
     -- Ok
     v := rvar.elem;
     v := rvar.child.elem;
     -- Not ok
     v := rvar.missing;
     v := rvar.child.missing;
  end process;

end architecture;
",
    );
    let diagnostics = builder.analyze();
    check_diagnostics(
        diagnostics,
        vec![
            Diagnostic::error(
                code.s("missing", 1),
                "No declaration of 'missing' within record type 'rec_t'",
            ),
            Diagnostic::error(
                code.s("missing", 2),
                "No declaration of 'missing' within record type 'rec_t'",
            ),
        ],
    );
}
