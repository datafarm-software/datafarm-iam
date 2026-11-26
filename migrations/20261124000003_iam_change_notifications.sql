-- =====================================================================
-- 3 of 3: change notifications (part of the public contract)
--
-- Depends on migration 1 (the triggers attach to the private iam tables).
-- Independent of migration 2, which has no effect on notifications.
--
-- Channel `iam_v1_changes`, delivered at commit. Payload is IDs only, e.g.
--   {"entity":"grant","op":"U","id":"<grant>","user_id":"<user>","node_id":"<node>"}
--   entity: node | user | device | grant        op: I | U | D
-- Consumers invalidate caches: grant and user events by user_id; node and device events
-- flush everything (placement changed, so any user's reachable devices may have changed).
-- Changing the payload shape means a new channel name, like a new schema version.
-- =====================================================================

CREATE FUNCTION iam.notify_change() RETURNS trigger
LANGUAGE plpgsql AS $$
DECLARE
  r record;
  j jsonb;
BEGIN
  IF TG_OP = 'DELETE' THEN r := OLD; ELSE r := NEW; END IF;
  j := to_jsonb(r);
  PERFORM pg_notify('iam_v1_changes', jsonb_strip_nulls(jsonb_build_object(
    'entity',    TG_ARGV[0],
    'op',        left(TG_OP, 1),
    'id',        j -> 'id',
    'user_id',   j -> 'user_id',
    'node_id',   j -> 'node_id',
    'device_id', j -> 'device_id'
  ))::text);
  RETURN NULL;
END $$;

-- Nodes notify only on create, delete and parent changes (renames and tag edits don't
-- affect access), so moving a subtree produces one notification, not one per descendant.
CREATE TRIGGER nodes_notify
  AFTER INSERT OR DELETE OR UPDATE OF parent_id ON iam.nodes
  FOR EACH ROW EXECUTE FUNCTION iam.notify_change('node');
CREATE TRIGGER users_notify
  AFTER INSERT OR DELETE OR UPDATE OF status, home_node_id ON iam.users
  FOR EACH ROW EXECUTE FUNCTION iam.notify_change('user');
CREATE TRIGGER devices_notify
  AFTER INSERT OR DELETE OR UPDATE OF node_id ON iam.devices
  FOR EACH ROW EXECUTE FUNCTION iam.notify_change('device');
CREATE TRIGGER grants_notify
  AFTER INSERT OR UPDATE OR DELETE ON iam.grants
  FOR EACH ROW EXECUTE FUNCTION iam.notify_change('grant');
