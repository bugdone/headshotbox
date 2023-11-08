export type DataTableHeader = {
  /**
   * Unique id, identifies column, (used by pagination.sortBy, 'body-cell-[name]' slot, ...)
   */
  name: string;

  /**
   * Label for header
   */
  label: string;

  /**
   * Row Object property to determine value for this column or function which maps to the required property
   */
  field: string | CallableFunction;

  /**
   * If we use visible-columns, this col will always be visible
   */
  required?: boolean;

  /**
   * Horizontal alignment of cells in this column
   */
  align?: 'left' | 'center' | 'right';

  /**
   * Tell QTable you want this column sortable
   */
  sortable?: boolean;

  /**
   * Compare function if you have some custom data or want a specific way to compare two rows
   */
  sort?: CallableFunction;

  /**
   * Set column sort order: 'ad' (ascending-descending) or 'da' (descending-ascending);
   * Overrides the 'column-sort-order' prop
   */
  sortOrder?: 'ad' | 'da';

  /**
   * Function you can apply to format your data
   */
  format?: CallableFunction;

  /**
   * Style to apply on normal cells of the column
   */
  style?: string | CallableFunction;

  /**
   * Classes to add on normal cells of the column
   */
  classes?: string | CallableFunction;

  /**
   * Style to apply on header cells of the column
   */
  headerStyle?: string;

  /**
   * Classes to add on header cells of the column
   */
  headerClasses?: string;

  /**
   * Determines whether the column would be visible besides the existing ones.
   */
  dynamic?: boolean;
};

export type DataTablePagination = {
  /**
   * Column name (from column definition)
   */
  sortBy?: string;
  /**
   * Is sorting in descending order?
   */
  descending?: boolean;
  /**
   * Page number (1-based)
   */
  page?: number;
  /**
   * How many rows per page? 0 means Infinite
   */
  rowsPerPage?: number;
  /**
   * For server-side fetching only. How many total database rows are there to be added to the table.
   * If set, causes the QTable to emit @request when data is required.
   */
  rowsNumber?: number;
};

export type DataTableRequestDetails = {
  pagination?: {
    sortBy: string;
    descending: boolean;
    page: number;
    rowsPerPage: number;
    rowsNumber: number;
  };
};
