---@meta

---@class Osm2pgsqlTable
local osm_table = {};

---Add data to a previously defined table
---@param row table<string, unknown>
---@return boolean, string, string, OsmObject
function osm_table:insert(row) end

---@class OsmGeometry
local osm_geometry = {}

---@return number
function osm_geometry:area() end

---@return OsmGeometry
function osm_geometry:centroid() end

---@return number, number, number, number
function osm_geometry:get_bbox() end

---@return unknown[]
function osm_geometry:geometries() end

---@param index integer
---@return OsmGeometry
function osm_geometry:geometry_n(index) end

---@return 'NULL' | 'POINT' | 'LINESTRING' | 'POLYGON' | 'MULTIPOINT' | 'MULTILINESTRING' | 'MULTIPOLYGON' | 'GEOMETRYCOLLECTION'
function osm_geometry:geometry_type() end

---@return boolean
function osm_geometry:is_null() end

---@return integer
function osm_geometry:length() end

---@return OsmGeometry
function osm_geometry:line_merge() end

---@return integer
function osm_geometry:num_geometries() end

---@param opts? OsmPoleOptions
---@return OsmGeometry
function osm_geometry:pole_of_inaccessibility(opts) end

---@param max_segment_length? number
---@return OsmGeometry
function osm_geometry:segmentize(max_segment_length) end

---@param tolerance? number
---@return OsmGeometry
function osm_geometry:simplify(tolerance) end

---@return number
function osm_geometry:spherical_area() end

---@return integer
function osm_geometry:srid() end

---@param target_srid? integer | string
---@return OsmGeometry
function osm_geometry:transform(target_srid) end

---@class OsmIdSpec
---@field type 'node' | 'way' | 'relation' | 'area' | 'any' | 'tile'
---@field id_column string
---@field create_index? 'auto' | 'always' | 'unique' | 'primary_key'

---@class OsmColumnDef
---@field column string
---@field type? 'text' | 'bool' | 'boolean' | 'int2' | 'smallint' | 'int4' | 'int' | 'integer' | 'int8' | 'bigint' | 'real' | 'hstore' | 'json' | 'jsonb' | 'direction' | 'geometry' | 'point' | 'linestring' | 'polygon' | 'multipoint' | 'multilinestring' | 'multipolygon' | 'geometrycollection'
---@field sql_type? string
---@field projection? integer | string
---@field not_null? boolean
---@field create_only? boolean
---@field expire? Expire

---@class Expire
---@field maxzoom? integer
---@field minzoom? integer
---@field filename? string
---@field schema? string
---@field table? string

---@class ExpireConfig
---@field maxzoom? integer
---@field minzoom? integer
---@field filename? string
---@field schema? string
---@field table? string

---@class Index
---@field column? string
---@field name? string
---@field expression? string
---@field include? string | string[]
---@field method? string
---@field tablespace? string
---@field unique? boolean
---@field where? string

---@class OsmDefineTypeTableOpts
---@field schema? string                    Target PostgreSQL schema.
---@field data_tablespace? string           Tablespace for table data.
---@field index_tablespace? string          Tablespace for table indexes.
---@field cluster? "auto"|"no"              Clustering strategy; defaults to "auto".
---@field indexes? Index[]                  Index definitions; defaults to a GIST on first geometry column.

---@class OsmDefineTableOpts: OsmDefineTypeTableOpts
---@field name string                       The name of the table (without schema).
---@field columns OsmColumnDef[]            Column definitions.
---@field ids? OsmIdSpec                    Id handling; tables without ids cannot be updated.

---@class OsmPoleOptions
---@field stretch? number

---@class OsmMember
---@field type 'n' | 'w' | 'r'
---@field ref string member ID
---@field role string

---@class OsmObject
---@field id integer
---@field type 'node' | 'way' | 'relation'
---@field tags table<string, string>
---@field version? integer
---@field timestamp? number
---@field changeset? integer
---@field uid? integer
---@field user? string
---@field is_closed boolean             Ways only: A boolean telling you whether the way geometry is closed, i.e. the first and last node are the same.
---@field nodes boolean                 Ways only: An array with the way node ids.
---@field members? OsmMember[]           Relations only: An array with member tables.
local osm_object = {}

---Create polygon geometry from OSM way object.
---@return OsmGeometry
function osm_object:as_polygon() end

---Create point geometry from OSM node object.
---@return OsmGeometry
function osm_object:as_point() end

---Create linestring geometry from OSM way object.
---@return OsmGeometry
function osm_object:as_linestring() end

---Create (multi)linestring geometry from OSM way/relation object.
---@return OsmGeometry
function osm_object:as_multilinestring() end

---Create (multi)polygon geometry from OSM way/relation object.
---@return OsmGeometry
function osm_object:as_multipolygon() end

---Create geometry collection from OSM relation object.
---@return OsmGeometry
function osm_object:as_geometrycollection() end

---@param key string
---@return string | nil
function osm_object:grab_tag(key) end

---@return number, number, number, number
function osm_object:get_bbox() end

---@class osm2pgsql
osm2pgsql = {}

---@param opts OsmDefineTableOpts
---@return Osm2pgsqlTable
function osm2pgsql.define_table(opts) end

---@param name string
---@param columns OsmColumnDef[]
---@param options OsmDefineTypeTableOpts
---@return Osm2pgsqlTable
function osm2pgsql.define_node_table(name, columns, options) end

---@param name string
---@param columns OsmColumnDef[]
---@param options OsmDefineTypeTableOpts
---@return Osm2pgsqlTable
function osm2pgsql.define_way_table(name, columns, options) end

---@param name string
---@param columns OsmColumnDef[]
---@param options OsmDefineTypeTableOpts
---@return Osm2pgsqlTable
function osm2pgsql.define_area_table(name, columns, options) end

---@param name string
---@param columns OsmColumnDef[]
---@param options OsmDefineTypeTableOpts
---@return Osm2pgsqlTable
function osm2pgsql.define_relation_table(name, columns, options) end

---@param object ExpireConfig
---@return Expire
function osm2pgsql.define_expire_output(object) end

---@param value number
---@param min number
---@param max number
---@return number
function osm2pgsql.clamp(value, min, max) end

---@param string string | nil
---@param prefix string
---@return boolean | nil
function osm2pgsql.has_prefix(string, prefix) end

---@param string string | nil
---@param suffix string
---@return boolean | nil
function osm2pgsql.has_suffix(string, suffix) end

---@param values string[]
---@param default? string
---@return fun(string: string | nil): string | nil
function osm2pgsql.make_check_values_func(values, default) end

---@param values string[]
---@return fun(tags: table<string, unknown>): nil
function osm2pgsql.make_clean_tags_func(values) end

---@param string string| nil
---@param delimiter? string
---@return string[] | nil
function osm2pgsql.split_string(string, delimiter) end

---@param string string | nil
---@param default_unit? string
---@return string | nil
function osm2pgsql.split_unit(string, default_unit) end

---@param string string: string | nil
---@return string | nil
function osm2pgsql.trim(string) end

---@param relation OsmObject
---@return integer[]
function osm2pgsql.node_member_ids(relation) end

---@param relation OsmObject
---@return integer[]
function osm2pgsql.way_member_ids(relation) end

---@type fun(object: OsmObject): nil
osm2pgsql.process_relation = nil

---Called by osm2pgsql for each node.
---@type fun(object: OsmObject): nil
osm2pgsql.process_node = nil

---Called by osm2pgsql for each way.
---@type fun(object: OsmObject): nil
osm2pgsql.process_way = nil

---@type fun(object: OsmObject): nil
osm2pgsql.process_untagged_node = nil

---@type fun(object: OsmObject): nil
osm2pgsql.process_untagged_way = nil

---@type fun(object: OsmObject): nil
osm2pgsql.process_untagged_relation = nil

---@type fun(object: OsmObject): nil
osm2pgsql.process_deleted_node = nil

---@type fun(object: OsmObject): nil
osm2pgsql.process_deleted_way = nil

---@type fun(object: OsmObject): nil
osm2pgsql.process_deleted_relation = nil
