# frozen_string_literal: true
require 'test/unit'

class TestFrozenChilledStrings < Test::Unit::TestCase
  def test_frozen_string_mutation
    str = "hello".freeze
    assert_true(str.frozen?)
    assert_raise(FrozenError) { str << " world" }
    assert_raise(FrozenError) { str.sub!("h", "H") }
    assert_raise(FrozenError) { str[0] = "H" }
  end

  def test_string_dup
    str = "hello".freeze
    dupped = str.dup
    assert_false(dupped.frozen?, "dup must return unfrozen copy")
    dupped << " world"
    assert_equal("hello world", dupped)
  end

  def test_string_clone_freeze_options
    str = "hello".freeze

    clone_frozen = str.clone(freeze: true)
    assert_true(clone_frozen.frozen?)
    assert_raise(FrozenError) { clone_frozen << " x" }

    clone_unfrozen = str.clone(freeze: false)
    assert_false(clone_unfrozen.frozen?)
    clone_unfrozen << " x"
    assert_equal("hello x", clone_unfrozen)

    unfrozen = +"hello"
    clone_default = unfrozen.clone
    assert_false(clone_default.frozen?)

    clone_frozen_explicit = unfrozen.clone(freeze: true)
    assert_true(clone_frozen_explicit.frozen?)
  end

  def test_immediate_object_ids
    assert_equal(nil.object_id, nil.object_id)
    assert_equal(true.object_id, true.object_id)
    assert_equal(false.object_id, false.object_id)
    assert_not_equal(nil.object_id, true.object_id)

    # Integer object_id consistency
    assert_equal(1.object_id, 1.object_id)
    assert_equal(1000.object_id, 1000.object_id)

    # Symbol object_id consistency
    assert_equal(:hello.object_id, :hello.object_id)
  end

  def test_heap_object_id_uniqueness
    obj1 = Object.new
    obj2 = Object.new
    assert_not_equal(obj1.object_id, obj2.object_id)

    s1 = +"hello"
    s2 = +"hello"
    assert_not_equal(s1.object_id, s2.object_id)
  end
end
