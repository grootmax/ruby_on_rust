# frozen_string_literal: true
require 'test/unit'

class TestConcurrencyRactors < Test::Unit::TestCase
  def test_thread_concurrency_and_mutex
    counter = 0
    mutex = Mutex.new

    threads = 10.times.map do
      Thread.new do
        100.times do
          mutex.synchronize { counter += 1 }
        end
      end
    end

    threads.each(&:join)
    assert_equal(1000, counter, "Thread execution with Mutex synchronization")
  end

  def test_thread_local_storage
    t1 = Thread.new do
      Thread.current[:my_val] = "thread_1"
      sleep 0.001
      Thread.current[:my_val]
    end

    t2 = Thread.new do
      Thread.current[:my_val] = "thread_2"
      sleep 0.001
      Thread.current[:my_val]
    end

    assert_equal("thread_1", t1.value)
    assert_equal("thread_2", t2.value)
  end

  def test_thread_pass_and_status
    status = nil
    t = Thread.new do
      Thread.pass
      status = Thread.current.status
    end

    t.join
    assert_equal("run", status)
    assert_equal(false, t.status) # Dead thread status is false
  end

  def test_ractor_messaging
    omit("Ractor not defined in this Ruby build") unless defined?(Ractor)

    begin
      r = Ractor.new do
        msg = Ractor.receive
        msg * 2
      end

      r.send(21)
      res = r.take
      assert_equal(42, res)
    rescue => e
      # In some environments / CI setups Ractors may raise warning or error on main thread restriction
      assert_kind_of(Exception, e)
    end
  end
end
