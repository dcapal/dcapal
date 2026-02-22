import { api } from "../httpClient";

export type ImportedPortfolio = Record<string, unknown>;

export const fetchImportedPortfolio = async (
  id: string
): Promise<ImportedPortfolio | null> => {
  const url = `/import/portfolio/${id}`;
  try {
    const response = await api.get<ImportedPortfolio>(url);

    if (response.status !== 200) {
      console.error(
        `Failed to fetch imported portfolio (${id}): {status: ${response.status}, data: ${response.data}}`
      );
      return null;
    }

    return response.data;
  } catch (error) {
    console.error(error);
    return null;
  }
};
